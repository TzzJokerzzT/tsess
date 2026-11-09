use std::path::{Path, PathBuf};

use crate::domain::config::{BUILTIN_LAYOUT_NAME, Settings, TsessConfig, builtin_dev_layout};
use crate::domain::error::ConfigError;
use crate::domain::project::{ResolvedRoot, SearchRoot};

use super::config_file;

/// Turn the global file plus the optional project file into resolved settings.
///
/// `project_dir` is `None` for commands that must not be influenced by the
/// current directory (for example `tsess list`).
pub fn resolve(
    config_file_path: &Path,
    project_dir: Option<&Path>,
) -> Result<Settings, ConfigError> {
    let mut sources: Vec<PathBuf> = Vec::new();
    let mut config = match config_file::load_optional(config_file_path)? {
        Some(config) => {
            sources.push(config_file_path.to_path_buf());
            config
        }
        None => TsessConfig::builtin(),
    };

    if let Some(dir) = project_dir
        && let Some(path) = config_file::project_config_path(dir)
    {
        let project = config_file::load_project(dir)?;
        sources.push(path);
        if let Some(project) = project {
            project.apply_to(&mut config);
        }
    }

    let default_layout = config
        .defaults
        .layout
        .clone()
        .unwrap_or_else(|| BUILTIN_LAYOUT_NAME.to_string());
    // `dev` is the documented fallback every configuration can rely on.
    config
        .layouts
        .entry(BUILTIN_LAYOUT_NAME.to_string())
        .or_insert_with(builtin_dev_layout);

    for (name, layout) in &config.layouts {
        layout
            .validate(name)
            .map_err(|source| ConfigError::InvalidLayout {
                name: name.clone(),
                source,
            })?;
    }
    if !config.layouts.contains_key(&default_layout) {
        return Err(ConfigError::UnknownDefaultLayout(default_layout));
    }

    let roots = expand_roots(&config.search.roots);
    Ok(Settings {
        search: config.search.clone(),
        roots,
        picker: config.picker,
        default_layout,
        layouts: config.layouts,
        sources,
    })
}

fn expand_roots(roots: &[SearchRoot]) -> Vec<ResolvedRoot> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut resolved = Vec::new();
    for root in roots {
        let raw = root.path.trim();
        if raw.is_empty() {
            continue;
        }
        let expanded = shellexpand::tilde(raw).to_string();
        let mut path = PathBuf::from(expanded);
        if !path.is_absolute() {
            let Ok(cwd) = std::env::current_dir() else {
                continue;
            };
            path = cwd.join(path);
        }
        if seen.contains(&path) {
            continue;
        }
        seen.push(path.clone());
        resolved.push(ResolvedRoot {
            path,
            depth: root.depth,
        });
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        let mut file = std::fs::File::create(path).expect("fixture created");
        file.write_all(body.as_bytes()).expect("fixture written");
    }

    #[test]
    fn without_any_file_the_builtin_dev_layout_is_the_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let settings = resolve(&dir.path().join("missing.toml"), None).expect("resolves");
        assert_eq!(settings.default_layout, BUILTIN_LAYOUT_NAME);
        assert!(settings.layouts.contains_key("dev"));
        assert!(settings.sources.is_empty());
    }

    #[test]
    fn the_project_default_layout_overrides_the_global_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global = dir.path().join("config.toml");
        write(&global, "[defaults]\nlayout = \"dev\"\n");
        let project_dir = dir.path().join("proj");
        write(
            &project_dir.join(".tsess.toml"),
            "[defaults]\nlayout = \"solo\"\n\n[layouts.solo]\nwindows = [{ name = \"editor\", panes = [{ cmd = \"nvim .\" }] }]\n",
        );

        let settings = resolve(&global, Some(&project_dir)).expect("resolves");
        assert_eq!(settings.default_layout, "solo");
        assert!(settings.layouts.contains_key("solo"));
        assert!(settings.layouts.contains_key("dev"));
        assert_eq!(settings.sources.len(), 2);
    }

    #[test]
    fn an_undefined_default_layout_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global = dir.path().join("config.toml");
        write(&global, "[defaults]\nlayout = \"nope\"\n");
        let error = resolve(&global, None).expect_err("unknown default layout");
        assert!(error.to_string().contains("nope"));
    }

    #[test]
    fn an_invalid_layout_names_the_layout_that_failed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global = dir.path().join("config.toml");
        write(&global, "[layouts.broken]\nwindows = []\n");
        let error = resolve(&global, None).expect_err("layout has no windows");
        assert!(error.to_string().contains("broken"));
    }

    #[test]
    fn roots_are_expanded_deduplicated_and_kept_absolute() {
        let dir = tempfile::tempdir().expect("tempdir");
        let global = dir.path().join("config.toml");
        write(
            &global,
            "[search]\nroots = [\n  { path = \"~/projects\", depth = 2 },\n  { path = \"~/projects\", depth = 3 },\n  { path = \"rel\", depth = 1 },\n]\n",
        );
        let settings = resolve(&global, None).expect("resolves");
        assert_eq!(settings.roots.len(), 2, "duplicate root is dropped");
        assert!(settings.roots[0].path.is_absolute());
        assert!(!settings.roots[0].path.to_string_lossy().starts_with('~'));
        assert_eq!(settings.roots[0].depth, 2);
        assert!(settings.roots[1].path.is_absolute());
        assert!(settings.roots[1].path.ends_with("rel"));
    }

    #[test]
    fn a_missing_global_file_still_reads_the_project_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project_dir = dir.path().join("proj");
        write(
            &project_dir.join(".tsess.toml"),
            "[defaults]\nlayout = \"dev\"\n",
        );
        let settings = resolve(&dir.path().join("missing.toml"), Some(&project_dir))
            .expect("resolves from the project file");
        assert_eq!(settings.default_layout, "dev");
        assert_eq!(settings.sources.len(), 1);
    }
}
