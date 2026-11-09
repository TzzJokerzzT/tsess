//! Interface layer: turns parsed arguments into application use cases and
//! renders their outcome for a human or a script.

use crate::application::{AppError, list::list_projects};
use crate::cli::{Args, Command};
use crate::domain::ports::ProjectScanner;
use crate::infrastructure::paths::AppPaths;
use crate::infrastructure::settings;
use crate::ui::text;

/// What happened, in terms the process can turn into an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The command exists in the CLI but this build does not run it yet.
    NotImplemented(&'static str),
}

pub fn run(
    args: &Args,
    paths: &AppPaths,
    scanner: &dyn ProjectScanner,
) -> Result<Outcome, AppError> {
    match &args.command {
        Some(Command::List) => list(paths, scanner),
        Some(Command::Init) => Ok(Outcome::NotImplemented("`tsess init`")),
        Some(Command::Open { .. }) => Ok(Outcome::NotImplemented("`tsess open`")),
        Some(Command::Last) => Ok(Outcome::NotImplemented("`tsess last`")),
        Some(Command::Kill { .. }) => Ok(Outcome::NotImplemented("`tsess kill`")),
        None => Ok(Outcome::NotImplemented("the interactive picker")),
    }
}

fn list(paths: &AppPaths, scanner: &dyn ProjectScanner) -> Result<Outcome, AppError> {
    // `list` must not be influenced by the configuration of whatever
    // directory the user happens to be standing in.
    let settings = settings::resolve(&paths.config_file, None)?;
    let projects = list_projects(scanner, &settings)?;
    tracing::info!(
        count = projects.len(),
        roots = settings.roots.len(),
        "discovered projects"
    );
    if projects.is_empty() {
        eprintln!("tsess: {}", text::empty_hint(&settings));
        return Ok(Outcome::Done);
    }
    print!("{}", text::project_lines(&projects));
    Ok(Outcome::Done)
}
