use super::error::DiscoveryError;
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
