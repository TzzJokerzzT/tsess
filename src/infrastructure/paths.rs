use std::path::{Path, PathBuf};

use directories::BaseDirs;
use thiserror::Error;

/// Where tsess keeps its configuration and runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub state_dir: PathBuf,
    pub log_file: PathBuf,
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("could not resolve your home directory; set HOME or the XDG variables")]
    NoHome,
}

impl AppPaths {
    /// `config_override` mirrors `--config`; everything else follows XDG.
    pub fn resolve_from(config_override: Option<&Path>) -> Result<Self, PathError> {
        let base = BaseDirs::new().ok_or(PathError::NoHome)?;
        let config_file = match config_override {
            Some(path) => path.to_path_buf(),
            None => base.config_dir().join("tsess").join("config.toml"),
        };
        let state_dir = base
            .state_dir()
            .map(|dir| dir.join("tsess"))
            .unwrap_or_else(|| base.home_dir().join(".local").join("state").join("tsess"));
        let log_file = state_dir.join("log").join("tsess.log");
        Ok(Self {
            config_file,
            state_dir,
            log_file,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_override_replaces_the_xdg_config_file() {
        let paths = AppPaths::resolve_from(Some(Path::new("/tmp/tsess-test.toml")))
            .expect("paths resolve with a home directory");
        assert_eq!(paths.config_file, PathBuf::from("/tmp/tsess-test.toml"));
    }

    #[test]
    fn the_default_config_file_lives_under_the_xdg_config_directory() {
        let paths = AppPaths::resolve_from(None).expect("paths resolve");
        assert!(paths.config_file.ends_with("tsess/config.toml"));
    }

    #[test]
    fn the_log_file_lives_under_the_state_directory() {
        let paths = AppPaths::resolve_from(None).expect("paths resolve");
        assert!(paths.log_file.starts_with(&paths.state_dir));
    }
}
