use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::error::StoreError;
use crate::domain::ports::ProjectConfigWriter;
use crate::infrastructure::config_file::{self, PROJECT_CONFIG};

/// The filesystem implementation of [`ProjectConfigWriter`].
#[derive(Debug, Default, Clone, Copy)]
pub struct ProjectFiles;

impl ProjectConfigWriter for ProjectFiles {
    fn existing(&self, dir: &Path) -> Option<PathBuf> {
        config_file::project_config_path(dir)
    }

    fn write_new(&self, dir: &Path, body: &str) -> Result<PathBuf, StoreError> {
        if let Some(existing) = config_file::project_config_path(dir) {
            return Err(StoreError::AlreadyExists(existing));
        }
        let target = dir.join(PROJECT_CONFIG);
        // `create_new` is the race-free form of "refuse to overwrite".
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|source| match source.kind() {
                std::io::ErrorKind::AlreadyExists => StoreError::AlreadyExists(target.clone()),
                _ => StoreError::Write {
                    path: target.clone(),
                    source,
                },
            })?;
        file.write_all(body.as_bytes())
            .map_err(|source| StoreError::Write {
                path: target.clone(),
                source,
            })?;
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_a_new_project_file_creates_it_and_refuses_a_second_time() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = ProjectFiles
            .write_new(dir.path(), "[defaults]\nlayout = \"dev\"\n")
            .expect("first write succeeds");
        assert_eq!(target, dir.path().join(PROJECT_CONFIG));
        assert_eq!(
            std::fs::read_to_string(&target).expect("file readable"),
            "[defaults]\nlayout = \"dev\"\n"
        );

        let error = ProjectFiles
            .write_new(dir.path(), "other")
            .expect_err("second write is refused");
        assert!(error.to_string().contains(".tsess.toml"));
        assert_eq!(
            std::fs::read_to_string(&target).expect("file readable"),
            "[defaults]\nlayout = \"dev\"\n",
            "a refused write must not modify the existing file"
        );
    }

    #[test]
    fn an_existing_legacy_file_also_blocks_a_new_canonical_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".sessionizer.toml"), "").expect("fixture");
        let existing = ProjectFiles.existing(dir.path()).expect("legacy file seen");
        assert!(existing.ends_with(".sessionizer.toml"));
        assert!(ProjectFiles.write_new(dir.path(), "").is_err());
        assert!(!dir.path().join(PROJECT_CONFIG).exists());
    }
}
