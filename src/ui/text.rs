//! Text rendering for the non-interactive commands.
//!
//! Output stays plain, sorted and tab-separated so it composes with `cut`,
//! `fzf`, or a status bar. Colors arrive with the theme system.

use crate::domain::config::Settings;
use crate::domain::project::Project;

/// One project per line: `name<TAB>path<TAB>markers`.
pub fn project_line(project: &Project) -> String {
    format!(
        "{}\t{}\t{}",
        project.name,
        project.display_path(),
        project.markers.join(",")
    )
}

/// All projects, newline terminated, sorted as the scanner returned them.
pub fn project_lines(projects: &[Project]) -> String {
    let mut out = String::new();
    for project in projects {
        out.push_str(&project_line(project));
        out.push('\n');
    }
    out
}

/// Explain an empty result with the roots that were actually searched.
pub fn empty_hint(settings: &Settings) -> String {
    let roots: Vec<String> = settings
        .roots
        .iter()
        .map(|root| root.path.display().to_string())
        .collect();
    if roots.is_empty() {
        return "no search roots configured; set [search] roots in your config".to_string();
    }
    format!(
        "no projects found under {} (markers: {})",
        roots.join(", "),
        settings.search.markers.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn project(name: &str, markers: &[&str]) -> Project {
        Project::new(
            PathBuf::from(format!("/srv/{name}")),
            markers.iter().map(|marker| marker.to_string()).collect(),
        )
    }

    #[test]
    fn a_project_line_is_tab_separated_and_scriptable() {
        let line = project_line(&project("api", &[".git", "Cargo.toml"]));
        assert_eq!(line, "api\t/srv/api\t.git,Cargo.toml");
    }

    #[test]
    fn project_lines_terminate_every_row() {
        let body = project_lines(&[project("a", &[".git"]), project("b", &["Cargo.toml"])]);
        assert_eq!(body, "a\t/srv/a\t.git\nb\t/srv/b\tCargo.toml\n");
    }

    #[test]
    fn listing_nothing_renders_nothing() {
        assert_eq!(project_lines(&[]), "");
    }

    #[test]
    fn the_empty_hint_names_the_searched_roots_and_markers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let settings =
            crate::infrastructure::settings::resolve(&dir.path().join("absent.toml"), None)
                .expect("settings resolve");
        let hint = empty_hint(&settings);
        assert!(hint.contains("no projects found under"));
        assert!(hint.contains(".git"));
    }
}
