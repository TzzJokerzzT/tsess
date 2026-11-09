//! File-based logging.
//!
//! A TUI owns the terminal, so nothing is ever written to stderr: the log goes
//! to a file under the state directory, created with mode 0600.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;

/// Environment variable that wins over `RUST_LOG`.
pub const LOG_ENV: &str = "TSESS_LOG";

#[derive(Debug, Error)]
pub enum LogError {
    #[error("could not create the log directory `{path}`: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not open the log file `{path}`: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("`{var}` is not a valid log filter: {source}")]
    Filter {
        var: &'static str,
        #[source]
        source: tracing_subscriber::filter::ParseError,
    },
}

/// Install the global subscriber. Never fails the process: callers report a
/// warning and keep running without logging.
pub fn init(log_file: &Path, verbosity: u8) -> Result<(), LogError> {
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LogError::Directory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(log_file)
        .map_err(|source| LogError::Open {
            path: log_file.to_path_buf(),
            source,
        })?;

    let filter = build_filter(verbosity)?;
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(true)
        .with_writer(FileMakeWriter::new(file));
    // A second initialisation (tests, embedded use) must not abort the process.
    let _ = subscriber.try_init();
    Ok(())
}

fn build_filter(verbosity: u8) -> Result<EnvFilter, LogError> {
    match std::env::var(LOG_ENV)
        .ok()
        .or_else(|| std::env::var("RUST_LOG").ok())
    {
        Some(raw) => EnvFilter::try_new(&raw).map_err(|source| LogError::Filter {
            var: LOG_ENV,
            source,
        }),
        None => {
            let level = match verbosity {
                0 => "info",
                1 => "debug",
                _ => "trace",
            };
            EnvFilter::try_new(level).map_err(|source| LogError::Filter {
                var: LOG_ENV,
                source,
            })
        }
    }
}

#[derive(Clone)]
struct FileMakeWriter {
    file: Arc<Mutex<File>>,
}

impl FileMakeWriter {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl<'a> MakeWriter<'a> for FileMakeWriter {
    type Writer = FileGuard;

    fn make_writer(&'a self) -> Self::Writer {
        FileGuard {
            file: Arc::clone(&self.file),
        }
    }
}

struct FileGuard {
    file: Arc<Mutex<File>>,
}

impl Write for FileGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.file.lock() {
            Ok(mut file) => file.write(buf),
            // Losing a log line must never crash the process.
            Err(_) => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.file.lock() {
            Ok(mut file) => file.flush(),
            Err(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_file_is_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("log").join("tsess.log");
        init(&path, 0).expect("logging initialises");

        let mode = std::fs::metadata(&path)
            .expect("log file exists")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn verbosity_selects_a_valid_filter_at_every_level() {
        for verbosity in 0..=2 {
            assert!(build_filter(verbosity).is_ok());
        }
    }
}
