# tsess

A tmux sessionizer in Rust: it scans your project folders, lets you fuzzy-pick one, and creates or
jumps to a tmux session built from a declarative layout (NeoVim on the left, the dev server on the
right, a terminal below).

```console
$ tsess                     # interactive floating picker
$ tsess open mi-api         # open or jump straight in
$ tsess list                # discovered projects + active sessions
$ tsess last                # toggle back to the previous session
$ tsess kill [nombre]       # close a session
$ tsess init                # write a .tsess.toml in the current project
```

## Status

This build is the foundation slice. Commands that do not work yet fail loudly with exit code 2
instead of pretending to succeed.

| Command | State |
| --- | --- |
| `tsess list` | projects work; the active-session column arrives with the tmux layer |
| `tsess init` | works |
| `tsess open`, `last`, `kill`, bare `tsess` | not implemented yet |

## Build

```console
$ cargo build --release
$ ./target/release/tsess list
```

Requires Rust 1.88 or newer. tmux 3.x is needed once the session layer lands.

## Configuration

Global configuration lives at `~/.config/tsess/config.toml` (respects `XDG_CONFIG_HOME`).
Every value below is optional; the defaults are shown by `tsess list` when nothing is configured.

```toml
[search]
roots = [
  { path = "~/Projects", depth = 2 },
  { path = "~/study",    depth = 1 },
]
markers = [".git", "package.json", "Cargo.toml"]  # what counts as a project
exclude = ["node_modules", "target", "dist"]

[picker]
backend = "builtin"   # "builtin" (ratatui) or "fzf"

[defaults]
layout = "dev"

[layouts.dev]
windows = [
  { name = "editor", panes = [{ cmd = "nvim ." }] },
  { name = "run", split = "vertical", panes = [{ cmd = "" }, { cmd = "" }] },
]
```

A project can override any of it with a `.tsess.toml` next to the code. Reading also accepts
`.sessionizer.toml` for projects that already use that name.

```toml
# .tsess.toml
[defaults]
layout = "api"

[layouts.api]
windows = [
  { name = "editor", panes = [{ cmd = "nvim ." }] },
  { name = "serve", split = "vertical", panes = [
      { cmd = "cargo watch -x run" },
      { cmd = "cargo test --watch" },
  ]},
]
```

`tsess init` generates that file seeded with the effective default layout and refuses to overwrite an
existing one.

### Layout vocabulary

| `split` | Meaning | tmux |
| --- | --- | --- |
| `vertical` | panes side by side (editor left, server right) | `split-window -h` |
| `horizontal` | panes stacked (terminal below) | `split-window -v` |

`vertical` follows the editor convention rather than tmux's own flag names, because the layout is
written from the developer's point of view.

## Discovery

Roots are walked with the [`ignore`](https://docs.rs/ignore) crate, so `.gitignore` rules apply even
before a project is a git repository. `depth` counts levels below the root: `depth = 2` means "a
project may sit two directories below the root". Duplicate paths from overlapping roots are reported
once. Results are sorted and tab separated:

```console
$ tsess list
alpha	~/Projects/alpha	.git
beta	~/Projects/beta	Cargo.toml
```

## Logging

Nothing is ever written to stderr: a TUI owns the terminal, so logs go to
`~/.local/state/tsess/log/tsess.log` (mode 0600).

```console
$ tsess -v list        # debug
$ tsess -vv list       # trace
$ TSESS_LOG="tsess=trace" tsess list
```

`TSESS_LOG` wins over `RUST_LOG`.

## Architecture

Clean Architecture, mirroring `omarchy-control-center`:

```
src/domain/          value types, port traits, typed errors (thiserror)
src/application/     use cases: list projects, init a project
src/adapters/        ports implemented: discovery (ignore), project file store
src/infrastructure/  XDG paths, config loading, file logging
src/ui/              rendering of command output
src/app/             turns parsed arguments into use cases
src/main.rs          the only anyhow boundary and the only wiring point
```

Anyhow appears only in `main.rs`; every other layer returns typed errors. External effects go
through ports so tests never touch a real tmux, network, or the user's home directory.

## Tests

```console
$ cargo test --all-targets
$ cargo clippy --all-targets -- -D warnings
```

Unit tests live inline per module; integration tests drive the real binary against a scratch
`HOME`, `XDG_CONFIG_HOME` and `XDG_STATE_HOME`.

## Roadmap

1. tmux adapter: `open`, `last`, `kill`, layout materialization, session runtime state.
2. Floating ratatui picker inside `tmux display-popup`, with preview, rename and session counter.
3. Layout editor: build and resize panes with the mouse or keyboard, with a save-target prompt.
4. Themes: the omarchy-control-center palette schema, builtin palettes and user themes.
