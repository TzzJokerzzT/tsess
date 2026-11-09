use thiserror::Error;

use crate::domain::error::ConfigError;
use crate::domain::error::DiscoveryError;

use super::init::InitError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error(transparent)]
    Init(#[from] InitError),
    #[error("could not resolve the current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}
