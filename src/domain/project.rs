use serde::{Deserialize, Serialize};

/// One configured search root, exactly as written under `[[search.roots]]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRoot {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
}

impl SearchRoot {
    pub fn new(path: impl Into<String>, depth: usize) -> Self {
        Self {
            path: path.into(),
            depth,
        }
    }
}

fn default_depth() -> usize {
    2
}

/// A search root with `~` already expanded, ready to be walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub path: std::path::PathBuf,
    pub depth: usize,
}

/// A directory that carries at least one project marker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    pub name: String,
    pub path: std::path::PathBuf,
    pub markers: Vec<String>,
}

impl Project {
    pub fn new(path: std::path::PathBuf, markers: Vec<String>) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string());
        Self {
            name,
            path,
            markers,
        }
    }

    /// The path with the home directory shortened to `~`, for human output.
    pub fn display_path(&self) -> String {
        abbreviate_home(&self.path)
    }
}

fn abbreviate_home(path: &std::path::Path) -> String {
    let Ok(home) = std::env::var("HOME") else {
        return path.display().to_string();
    };
    if home.is_empty() {
        return path.display().to_string();
    }
    let home = std::path::Path::new(&home);
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn a_project_takes_its_name_from_the_directory() {
        let project = Project::new(PathBuf::from("/tmp/work/mi-api"), vec![".git".to_string()]);
        assert_eq!(project.name, "mi-api");
    }

    #[test]
    fn display_path_shortens_the_home_directory() {
        // SAFETY: this test only reads HOME back through the same process.
        unsafe { std::env::set_var("HOME", "/home/tester") };
        let project = Project::new(PathBuf::from("/home/tester/work/api"), Vec::new());
        assert_eq!(project.display_path(), "~/work/api");
    }

    #[test]
    fn display_path_leaves_paths_outside_home_alone() {
        // SAFETY: this test only reads HOME back through the same process.
        unsafe { std::env::set_var("HOME", "/home/tester") };
        let project = Project::new(PathBuf::from("/srv/api"), Vec::new());
        assert_eq!(project.display_path(), "/srv/api");
    }
}
