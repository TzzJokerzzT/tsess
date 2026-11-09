use crate::domain::config::Settings;
use crate::domain::error::DiscoveryError;
use crate::domain::ports::ProjectScanner;
use crate::domain::project::Project;

/// Discover the projects declared by the effective configuration.
pub fn list_projects(
    scanner: &dyn ProjectScanner,
    settings: &Settings,
) -> Result<Vec<Project>, DiscoveryError> {
    scanner.scan(
        settings.scanned_roots(),
        &settings.search.markers,
        &settings.search.exclude,
    )
}
