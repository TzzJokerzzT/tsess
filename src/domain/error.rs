use std::path::PathBuf;

use thiserror::Error;

/// Why a layout cannot be materialized into tmux windows and panes.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LayoutError {
    #[error("layout name cannot be empty")]
    EmptyName,
    #[error("layout name `{0}` cannot contain whitespace")]
    WhitespaceInName(String),
    #[error("layout `{0}` declares no windows")]
    NoWindows(String),
    #[error("window `{window}` in layout `{layout}` cannot be split into fewer than two panes")]
    SplitWithOnePane { layout: String, window: String },
    #[error("window name cannot be empty in layout `{0}`")]
    EmptyWindowName(String),
}

/// Why scanning the configured roots failed.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("could not walk `{path}`: {message}")]
    Walk { path: PathBuf, message: String },
}

/// Why a configuration file could not be turned into usable settings.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration `{path}` is not valid TOML: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("could not read configuration `{path}`: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("layout `{name}` is invalid: {source}")]
    InvalidLayout {
        name: String,
        #[source]
        source: LayoutError,
    },
    #[error("default layout `{0}` is not defined under [layouts.*]")]
    UnknownDefaultLayout(String),
}
