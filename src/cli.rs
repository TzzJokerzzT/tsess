use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

/// Command line surface for `tsess`.
#[derive(Debug, Parser)]
#[command(
    name = "tsess",
    version,
    about = "Pick a project, get a tmux session with your layout",
    long_about = None
)]
pub struct Args {
    /// Raise log verbosity: -v for debug, -vv for trace
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    /// Config file to use instead of the XDG default
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open a session for a project, or jump to it when it already exists
    Open {
        /// Project path or project name
        target: String,
    },
    /// List discovered projects and active sessions
    List,
    /// Switch back to the previous session
    Last,
    /// Kill a session
    Kill {
        /// Session name; the picker is used when omitted
        name: Option<String>,
    },
    /// Write a .tsess.toml in the current project
    Init,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_subcommand_means_the_interactive_picker() {
        let args = Args::try_parse_from(["tsess"]).expect("bare tsess parses");
        assert!(args.command.is_none());
    }

    #[test]
    fn open_takes_a_target() {
        let args = Args::try_parse_from(["tsess", "open", "mi-api"]).expect("open parses");
        assert!(matches!(
            args.command,
            Some(Command::Open { ref target }) if target == "mi-api"
        ));
    }

    #[test]
    fn kill_takes_an_optional_name() {
        let bare = Args::try_parse_from(["tsess", "kill"]).expect("kill parses");
        assert!(matches!(bare.command, Some(Command::Kill { name: None })));
        let named = Args::try_parse_from(["tsess", "kill", "api"]).expect("kill parses");
        assert!(matches!(
            named.command,
            Some(Command::Kill { name: Some(ref name) }) if name == "api"
        ));
    }

    #[test]
    fn verbosity_accumulates_and_config_is_global() {
        let args = Args::try_parse_from(["tsess", "-vv", "--config", "/tmp/c.toml", "list"])
            .expect("flags parse");
        assert_eq!(args.verbose, 2);
        assert_eq!(
            args.config.as_deref(),
            Some(std::path::Path::new("/tmp/c.toml"))
        );
        assert!(matches!(args.command, Some(Command::List)));
    }
}
