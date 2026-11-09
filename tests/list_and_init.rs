//! End-to-end checks against the real binary with a scratch HOME, XDG config
//! and state, and a throwaway project tree.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Scratch {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("config/tsess")).expect("config dir");
        std::fs::create_dir_all(root.join("state")).expect("state dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        Self { _dir: dir, root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn touch(&self, relative: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, b"").expect("file");
    }

    fn write_config(&self, roots_depth: usize) {
        let projects = self.path("projects");
        let body = format!(
            r#"
[search]
roots = [{{ path = "{}", depth = {roots_depth} }}]
markers = [".git", "Cargo.toml", "package.json"]
exclude = ["node_modules", "target", "dist"]

[defaults]
layout = "dev"

[layouts.dev]
windows = [
  {{ name = "editor", panes = [{{ cmd = "nvim ." }}] }},
  {{ name = "run", split = "vertical", panes = [{{ cmd = "" }}, {{ cmd = "" }}] }},
]
"#,
            projects.display()
        );
        std::fs::write(self.path("config/tsess/config.toml"), body).expect("config written");
    }

    fn project_tree(&self) {
        self.touch("projects/alpha/.git/HEAD");
        self.touch("projects/beta/Cargo.toml");
        self.touch("projects/beta/node_modules/dep/package.json");
        self.touch("projects/notes/readme.md");
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tsess"));
        command
            .env("XDG_CONFIG_HOME", self.path("config"))
            .env("XDG_STATE_HOME", self.path("state"))
            .env("HOME", self.path("home"))
            .env("TSESS_LOG", "debug");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("binary runs")
    }

    fn run_in(&self, dir: &Path, args: &[&str]) -> Output {
        self.command()
            .current_dir(dir)
            .args(args)
            .output()
            .expect("binary runs")
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn list_prints_discovered_projects_and_skips_excluded_directories() {
    let scratch = Scratch::new();
    scratch.project_tree();
    scratch.write_config(3);

    let output = scratch.run(&["list"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let body = stdout(&output);
    let rows: Vec<&str> = body.lines().collect();
    assert_eq!(rows.len(), 2, "stdout was: {body}");
    assert!(rows[0].starts_with("alpha\t"), "stdout was: {body}");
    assert!(rows[0].ends_with(".git"));
    assert!(rows[1].starts_with("beta\t"), "stdout was: {body}");
    assert!(rows[1].ends_with("Cargo.toml"));
    assert!(!body.contains("dep"), "excluded tree leaked: {body}");
    assert!(!body.contains("notes"), "non-project leaked: {body}");
}

#[test]
fn list_without_a_configuration_uses_defaults_and_stays_successful() {
    let scratch = Scratch::new();
    let output = scratch.run(&["list"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("no projects found"));
}

#[test]
fn list_reports_a_broken_configuration_with_a_failing_exit() {
    let scratch = Scratch::new();
    std::fs::write(
        scratch.path("config/tsess/config.toml"),
        "[picker]\nbackend = \"dmenu\"\n",
    )
    .expect("config written");

    let output = scratch.run(&["list"]);
    assert!(!output.status.success());
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("config.toml"));
}

#[test]
fn init_creates_the_project_file_once_and_refuses_to_overwrite_it() {
    let scratch = Scratch::new();
    let project = scratch.path("projects/gamma");
    std::fs::create_dir_all(&project).expect("project dir");

    let first = scratch.run_in(&project, &["init"]);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    let config = project.join(".tsess.toml");
    let written = std::fs::read_to_string(&config).expect("project file exists");
    assert!(written.contains("[[layouts.dev.windows]]"));
    assert!(written.contains("nvim ."));

    let second = scratch.run_in(&project, &["init"]);
    assert!(!second.status.success(), "init must refuse a second write");
    assert!(stderr(&second).contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(&config).expect("project file still readable"),
        written,
        "a refused init must leave the file untouched"
    );
}

#[test]
fn commands_that_are_not_implemented_yet_fail_loudly_and_print_nothing_on_stdout() {
    let scratch = Scratch::new();
    for args in [
        vec!["open", "alpha"],
        vec!["last"],
        vec!["kill"],
        vec!["kill", "alpha"],
        vec![],
    ] {
        let output = scratch.run(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "args {args:?} stderr: {}",
            stderr(&output)
        );
        assert!(
            stdout(&output).is_empty(),
            "args {args:?} leaked stdout: {}",
            stdout(&output)
        );
        assert!(
            stderr(&output).contains("not available in this build yet"),
            "args {args:?} stderr: {}",
            stderr(&output)
        );
    }
}

#[test]
fn the_config_override_flag_replaces_the_xdg_configuration() {
    let scratch = Scratch::new();
    scratch.project_tree();
    let custom = scratch.path("custom.toml");
    std::fs::write(
        &custom,
        format!(
            "[search]\nroots = [{{ path = \"{}\", depth = 1 }}]\nmarkers = [\"Cargo.toml\"]\n",
            scratch.path("projects").display()
        ),
    )
    .expect("custom config written");

    let output = scratch.run(&["list", "--config", custom.to_str().expect("utf-8")]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let body = stdout(&output);
    assert!(body.starts_with("beta\t"), "stdout was: {body}");
    assert!(!body.contains("alpha"), "stdout was: {body}");
}
