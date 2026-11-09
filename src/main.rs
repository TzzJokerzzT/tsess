use std::process::ExitCode;

use clap::Parser;

mod adapters;
mod app;
mod application;
mod cli;
mod domain;
mod infrastructure;
mod ui;

use crate::app::Outcome;
use crate::cli::Args;
use crate::infrastructure::paths::AppPaths;

fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("tsess: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// The single `anyhow` boundary: everything below returns typed errors.
fn run(args: Args) -> anyhow::Result<ExitCode> {
    let paths = AppPaths::resolve_from(args.config.as_deref())?;
    if let Err(error) = infrastructure::logging::init(&paths.log_file, args.verbose) {
        // Logging is a convenience: never let it break the command.
        eprintln!("tsess: warning: file logging disabled: {error}");
    }
    tracing::info!(
        config = %paths.config_file.display(),
        state = %paths.state_dir.display(),
        "tsess started"
    );

    let scanner = adapters::IgnoreScanner;
    let files = adapters::ProjectFiles;

    match app::run(&args, &paths, &scanner, &files)? {
        Outcome::Done => Ok(ExitCode::SUCCESS),
        Outcome::NotImplemented(what) => {
            eprintln!("tsess: {what} is not available in this build yet");
            Ok(ExitCode::from(2))
        }
    }
}
