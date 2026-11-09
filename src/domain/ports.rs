use std::path::{Path, PathBuf};

use super::error::DiscoveryError;
use super::error::StoreError;
use super::project::{Project, ResolvedRoot};

/// Finds project directories under the configured roots.
///
/// Kept as a port so tests and future backends never shell out or touch the
/// user's real tree.
pub trait ProjectScanner: Send + Sync {
    fn scan(
        &self,
        roots: &[ResolvedRoot],
        markers: &[String],
        exclude: &[String],
    ) -> Result<Vec<Project>, DiscoveryError>;
}

/// Reads and writes a project's `.tsess.toml`.
pub trait ProjectConfigWriter: Send + Sync {
    /// The project configuration file already present in `dir`, if any.
    fn existing(&self, dir: &Path) -> Option<PathBuf>;

    /// Create a new project configuration file, never overwriting one.
    fn write_new(&self, dir: &Path, body: &str) -> Result<PathBuf, StoreError>;
}
