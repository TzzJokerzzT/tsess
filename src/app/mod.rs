//! Interface layer: turns parsed arguments into application use cases and
//! renders their outcome for a human or a script.

use crate::application::{AppError, init::init_in, list::list_projects};
use crate::cli::{Args, Command};
use crate::domain::ports::{ProjectConfigWriter, ProjectScanner};
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
    files: &dyn ProjectConfigWriter,
) -> Result<Outcome, AppError> {
    match &args.command {
        Some(Command::List) => list(paths, scanner),
        Some(Command::Init) => init(paths, files),
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

fn init(paths: &AppPaths, files: &dyn ProjectConfigWriter) -> Result<Outcome, AppError> {
    let cwd = std::env::current_dir()?;
    let settings = settings::resolve(&paths.config_file, None)?;
    let target = init_in(&cwd, &settings, files)?;
    tracing::info!(path = %target.display(), "wrote project configuration");
    println!("wrote {}", target.display());
    Ok(Outcome::Done)
}
