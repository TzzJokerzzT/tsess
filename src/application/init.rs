use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::domain::config::{DefaultsSection, ProjectConfig, Settings, render_project_config};
use crate::domain::layout::Layouts;
use crate::domain::ports::ProjectConfigWriter;

#[derive(Debug, Error)]
pub enum InitError {
    #[error("`{0}` already exists; refusing to overwrite it")]
    AlreadyExists(PathBuf),
    #[error("could not render the generated configuration: {0}")]
    Render(#[from] toml::ser::Error),
    #[error(transparent)]
    Store(#[from] crate::domain::error::StoreError),
}

/// Write a `.tsess.toml` into `dir` seeded with the effective default layout.
pub fn init_in(
    dir: &Path,
    settings: &Settings,
    store: &dyn ProjectConfigWriter,
) -> Result<PathBuf, InitError> {
    if let Some(existing) = store.existing(dir) {
        return Err(InitError::AlreadyExists(existing));
    }
    let name = settings.default_layout.clone();
    let mut layouts = Layouts::new();
    layouts.insert(name.clone(), settings.default_layout_definition().clone());
    let config = ProjectConfig {
        picker: None,
        defaults: Some(DefaultsSection { layout: Some(name) }),
        layouts,
    };
    let body = render_project_config(&config)?;
    Ok(store.write_new(dir, &body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::StoreError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeStore {
        written: Mutex<Vec<(PathBuf, String)>>,
        existing: Option<PathBuf>,
    }

    impl ProjectConfigWriter for FakeStore {
        fn existing(&self, _dir: &Path) -> Option<PathBuf> {
            self.existing.clone()
        }

        fn write_new(&self, dir: &Path, body: &str) -> Result<PathBuf, StoreError> {
            if let Some(existing) = self.existing.clone() {
                return Err(StoreError::AlreadyExists(existing));
            }
            self.written
                .lock()
                .expect("lock")
                .push((dir.to_path_buf(), body.to_string()));
            Ok(dir.join(".tsess.toml"))
        }
    }

    fn settings() -> Settings {
        let dir = tempfile::tempdir().expect("tempdir");
        crate::infrastructure::settings::resolve(&dir.path().join("missing.toml"), None)
            .expect("builtin settings")
    }

    #[test]
    fn init_writes_the_default_layout_into_the_project() {
        let store = FakeStore::default();
        let target = init_in(Path::new("/tmp/proj"), &settings(), &store).expect("init succeeds");
        assert_eq!(target, Path::new("/tmp/proj/.tsess.toml"));

        let written = store.written.lock().expect("lock");
        assert_eq!(written.len(), 1);
        let body = &written[0].1;
        assert!(body.contains("[[layouts.dev.windows]]"));
        assert!(body.contains("nvim ."));
        assert!(body.contains("split = \"vertical\""));
        let parsed: ProjectConfig = toml::from_str(body).expect("generated file round trips");
        assert_eq!(
            parsed.defaults.and_then(|d| d.layout).as_deref(),
            Some("dev")
        );
    }

    #[test]
    fn init_refuses_an_existing_project_file_and_writes_nothing() {
        let store = FakeStore {
            existing: Some(PathBuf::from("/tmp/proj/.tsess.toml")),
            ..FakeStore::default()
        };
        let error = init_in(Path::new("/tmp/proj"), &settings(), &store)
            .expect_err("an existing project file is refused");
        assert!(error.to_string().contains("/tmp/proj/.tsess.toml"));
        assert!(
            store.written.lock().expect("lock").is_empty(),
            "a refused init must not write anything"
        );
    }
}
