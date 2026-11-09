use thiserror::Error;

use crate::domain::error::ConfigError;
use crate::domain::error::DiscoveryError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error("could not resolve the current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}
