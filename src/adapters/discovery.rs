use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::domain::error::DiscoveryError;
use crate::domain::ports::ProjectScanner;
use crate::domain::project::{Project, ResolvedRoot};

/// Scans the configured roots with the `ignore` crate, so `.gitignore` rules,
/// the configured `exclude` names, and the per-root depth are all honored.
///
/// A directory is a project when it *contains* at least one marker. The walk
/// therefore visits directories and probes for markers, instead of hunting for
/// marker files: the walker hides dotfiles by default, and `.git` is both a
/// marker and a directory we never want to descend into.
#[derive(Debug, Default, Clone, Copy)]
pub struct IgnoreScanner;

impl ProjectScanner for IgnoreScanner {
    fn scan(
        &self,
        roots: &[ResolvedRoot],
        markers: &[String],
        exclude: &[String],
    ) -> Result<Vec<Project>, DiscoveryError> {
        let mut found: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

        for root in roots {
            match std::fs::metadata(&root.path) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    tracing::debug!(
                        root = %root.path.display(),
                        "search root is not a directory; skipping"
                    );
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tracing::debug!(
                        root = %root.path.display(),
                        "search root does not exist; skipping"
                    );
                    continue;
                }
                Err(error) => {
                    // A root that exists but cannot be inspected is a real
                    // misconfiguration; report it instead of silently listing nothing.
                    return Err(DiscoveryError::Walk {
                        path: root.path.clone(),
                        message: error.to_string(),
                    });
                }
            }

            let exclude_owned = exclude.to_vec();
            let mut builder = WalkBuilder::new(&root.path);
            builder
                .max_depth(Some(root.depth))
                .follow_links(false)
                // Honor a `.gitignore` even when the tree is not a git
                // repository yet; respecting the user's ignore rules is the
                // point, not whether `git init` ran.
                .require_git(false);
            builder
                .filter_entry(move |entry| keep_entry(entry.path(), entry.depth(), &exclude_owned));

            for item in builder.build() {
                let entry = match item {
                    Ok(entry) => entry,
                    Err(error) => {
                        tracing::debug!(%error, root = %root.path.display(), "walk error");
                        continue;
                    }
                };
                if !entry.file_type().is_some_and(|kind| kind.is_dir()) {
                    continue;
                }
                let dir = entry.path();
                let matched: Vec<String> = markers
                    .iter()
                    .filter(|marker| !marker.is_empty() && dir.join(marker).exists())
                    .cloned()
                    .collect();
                if matched.is_empty() {
                    continue;
                }
                found.entry(dir.to_path_buf()).or_default().extend(matched);
            }
        }

        let mut projects: Vec<Project> = found
            .into_iter()
            .map(|(dir, mut markers)| {
                markers.sort();
                markers.dedup();
                Project::new(dir, markers)
            })
            .collect();
        projects.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.path.cmp(&right.path))
        });
        Ok(projects)
    }
}

/// Drop the configured `exclude` directory names from the walk.
fn keep_entry(path: &Path, depth: usize, exclude: &[String]) -> bool {
    if depth == 0 {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !exclude.iter().any(|excluded| excluded == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(path: &Path, depth: usize) -> ResolvedRoot {
        ResolvedRoot {
            path: path.to_path_buf(),
            depth,
        }
    }

    fn markers() -> Vec<String> {
        [".git", "Cargo.toml", "package.json"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn exclude() -> Vec<String> {
        ["node_modules", "target", "dist"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, b"").expect("file");
    }

    fn names(projects: &[Project]) -> Vec<&str> {
        projects
            .iter()
            .map(|project| project.name.as_str())
            .collect()
    }

    #[test]
    fn a_directory_with_any_marker_is_a_project() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        touch(&base.join("gitty/.git/HEAD"));
        touch(&base.join("cargo-app/Cargo.toml"));
        touch(&base.join("node-app/package.json"));
        touch(&base.join("notes/readme.md"));

        let projects = IgnoreScanner
            .scan(&[root(base, 2)], &markers(), &exclude())
            .expect("scan succeeds");

        assert_eq!(names(&projects), ["cargo-app", "gitty", "node-app"]);
        assert_eq!(projects[1].markers, vec![".git".to_string()]);
    }

    #[test]
    fn the_configured_depth_is_respected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        touch(&base.join("level1/app/Cargo.toml"));

        let shallow = IgnoreScanner
            .scan(&[root(base, 1)], &markers(), &exclude())
            .expect("scan succeeds");
        assert!(shallow.is_empty(), "depth 1 must not see level1/app");

        let deep = IgnoreScanner
            .scan(&[root(base, 2)], &markers(), &exclude())
            .expect("scan succeeds");
        assert_eq!(names(&deep), ["app"]);
    }

    #[test]
    fn excluded_directories_are_not_descended_into() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        touch(&base.join("web/node_modules/dep/package.json"));
        touch(&base.join("web/package.json"));

        let projects = IgnoreScanner
            .scan(&[root(base, 3)], &markers(), &exclude())
            .expect("scan succeeds");

        assert_eq!(names(&projects), ["web"]);
    }

    #[test]
    fn a_gitignored_directory_is_not_descended_into() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        std::fs::write(base.join(".gitignore"), "generated/\n").expect("gitignore");
        touch(&base.join("generated/app/Cargo.toml"));
        touch(&base.join("kept/Cargo.toml"));

        let projects = IgnoreScanner
            .scan(&[root(base, 3)], &markers(), &exclude())
            .expect("scan succeeds");

        assert_eq!(names(&projects), ["kept"]);
    }

    #[test]
    fn overlapping_roots_never_report_the_same_project_twice() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        touch(&base.join("app/Cargo.toml"));

        let projects = IgnoreScanner
            .scan(
                &[root(base, 2), root(&base.join("app"), 1)],
                &markers(),
                &exclude(),
            )
            .expect("scan succeeds");

        assert_eq!(names(&projects), ["app"]);
    }

    #[test]
    fn every_marker_found_in_a_project_is_reported_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();
        touch(&base.join("mono/.git/HEAD"));
        touch(&base.join("mono/Cargo.toml"));
        touch(&base.join("mono/package.json"));

        let projects = IgnoreScanner
            .scan(&[root(base, 2)], &markers(), &exclude())
            .expect("scan succeeds");

        assert_eq!(
            projects[0].markers,
            vec![
                ".git".to_string(),
                "Cargo.toml".to_string(),
                "package.json".to_string()
            ]
        );
    }

    #[test]
    fn a_missing_root_is_skipped_without_failing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("nope");
        let projects = IgnoreScanner
            .scan(&[root(&missing, 2)], &markers(), &exclude())
            .expect("scan succeeds");
        assert!(projects.is_empty());
    }
}
