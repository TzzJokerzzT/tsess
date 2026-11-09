use std::path::{Path, PathBuf};

use crate::domain::config::{ProjectConfig, TsessConfig};
use crate::domain::error::ConfigError;

/// Canonical project configuration file name.
pub const PROJECT_CONFIG: &str = ".tsess.toml";
/// Accepted as a read-only fallback for projects that already use the older name.
pub const LEGACY_PROJECT_CONFIG: &str = ".sessionizer.toml";

/// Read the global configuration. A missing file is not an error.
pub fn load_optional(path: &Path) -> Result<Option<TsessConfig>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let config = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
            Ok(Some(config))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Read a project's configuration, preferring `.tsess.toml` and falling back to
/// the legacy `.sessionizer.toml`.
pub fn load_project(dir: &Path) -> Result<Option<ProjectConfig>, ConfigError> {
    let Some(path) = project_config_path(dir) else {
        return Ok(None);
    };
    let raw = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    let config = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.clone(),
        source,
    })?;
    Ok(Some(config))
}

/// Which project configuration file exists in `dir`, if any.
pub fn project_config_path(dir: &Path) -> Option<PathBuf> {
    for candidate in [PROJECT_CONFIG, LEGACY_PROJECT_CONFIG] {
        let path = dir.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::PickerBackend;
    use crate::domain::layout::Split;
    use std::io::Write;

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).expect("fixture created");
        file.write_all(body.as_bytes()).expect("fixture written");
        path
    }

    const USER_CONFIG: &str = r#"
[search]
roots = [
  { path = "~/projects", depth = 2 },
  { path = "~/study",    depth = 1 },
]
markers = [".git", "package.json", "Cargo.toml"]  # qué cuenta como proyecto
exclude = ["node_modules", "target", "dist"]

[picker]
backend = "fzf"   # "fzf" en el MVP, "builtin" después

[defaults]
layout = "dev"

[layouts.dev]
windows = [
  { name = "editor", panes = [{ cmd = "nvim ." }] },
  { name = "run", split = "vertical", panes = [{ cmd = "" }, { cmd = "" }] },
]
"#;

    #[test]
    fn the_documented_global_configuration_parses_into_the_expected_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "config.toml", USER_CONFIG);
        let config = load_optional(&path)
            .expect("configuration loads")
            .expect("configuration exists");

        assert_eq!(config.search.roots.len(), 2);
        assert_eq!(config.search.roots[0].path, "~/projects");
        assert_eq!(config.search.roots[0].depth, 2);
        assert_eq!(config.search.roots[1].depth, 1);
        assert_eq!(
            config.search.markers,
            [".git", "package.json", "Cargo.toml"]
        );
        assert_eq!(config.search.exclude, ["node_modules", "target", "dist"]);
        assert_eq!(config.picker.backend, PickerBackend::Fzf);
        assert_eq!(config.defaults.layout.as_deref(), Some("dev"));

        let dev = config.layouts.get("dev").expect("dev layout");
        assert_eq!(dev.windows.len(), 2);
        assert_eq!(dev.windows[0].name, "editor");
        assert_eq!(dev.windows[0].panes[0].cmd, "nvim .");
        assert_eq!(dev.windows[1].split, Some(Split::Vertical));
        assert_eq!(dev.windows[1].panes.len(), 2);
        assert_eq!(dev.validate("dev"), Ok(()));
    }

    #[test]
    fn a_missing_configuration_file_is_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            load_optional(&dir.path().join("absent.toml"))
                .expect("no error")
                .is_none()
        );
    }

    #[test]
    fn a_malformed_configuration_names_the_offending_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "config.toml", "[picker]\nbackend = \"dmenu\"\n");
        let error = load_optional(&path).expect_err("backend is not a known value");
        let rendered = error.to_string();
        assert!(rendered.contains("config.toml"), "error was: {rendered}");
    }

    #[test]
    fn an_unknown_section_is_rejected_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write(dir.path(), "config.toml", "[themez]\nfondo = \"#000000\"\n");
        let error = load_optional(&path).expect_err("unknown sections are rejected");
        assert!(error.to_string().contains("themez"));
    }

    #[test]
    fn a_project_prefers_the_canonical_file_and_accepts_the_legacy_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            LEGACY_PROJECT_CONFIG,
            "[defaults]\nlayout = \"legacy\"\n",
        );
        assert_eq!(
            project_config_path(dir.path()),
            Some(dir.path().join(LEGACY_PROJECT_CONFIG))
        );

        write(
            dir.path(),
            PROJECT_CONFIG,
            "[defaults]\nlayout = \"canonical\"\n",
        );
        let path = project_config_path(dir.path()).expect("canonical file wins");
        assert_eq!(path, dir.path().join(PROJECT_CONFIG));
        let project = load_project(dir.path())
            .expect("project loads")
            .expect("project exists");
        assert_eq!(
            project
                .defaults
                .and_then(|defaults| defaults.layout)
                .as_deref(),
            Some("canonical")
        );
    }
}
