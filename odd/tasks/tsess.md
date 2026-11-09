# tsess — tmux session manager (Rust, Clean Architecture)

Greenfield Rust CLI + TUI that scans project roots, shows a fuzzy picker, and creates or jumps to a
tmux session using a declarative layout. Delivered in reviewable slices (S1-S4 of the phasing plan are
tasks T1..T5 below). Architecture mirrors `omarchy-control-center`: `domain` / `application` /
`adapters` / `infrastructure` / `ui` / `app`, ports as sync traits, `thiserror` inside the layers,
`anyhow` only in `main.rs`.

## Specs

S1. Command surface. These exact command lines, verbatim:

```
tsess                 
# picker interactivo (caso principal)
tsess open <ruta|nombre>   # abre o salta directo
tsess list             # proyectos descubiertos + sesiones activas
tsess last             # alterna con la sesión anterior
tsess kill [nombre]    # cierra una sesión
tsess init             # genera un .tsess.toml en el proyecto actual
```

Bare `tsess` with no subcommand is the interactive picker.

S2. Global configuration lives under XDG config and uses exactly this shape (user's example, verbatim):

```toml
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
```

Because of decision D1/D2 the effective default for `[picker] backend` is `"builtin"`; `"fzf"` stays
supported as an opt-in backend. Every other field keeps the user's names and defaults.

S3. Per-project file `.tsess.toml` defines that project's panels and commands. Reading also accepts
`.sessionizer.toml` as a fallback name. `tsess init` generates `.tsess.toml` in the current project
from the global defaults when absent, and refuses to overwrite an existing file.

S4. Default dev layout intent: NeoVim on the left, the dev server on the right, a terminal below.
Expressed with the S2 `[layouts.*]` vocabulary — no hardcoded layout in the binary beyond the
`dev` default used when no config exists.

S5. Picker requirements: it is a fuzzy finder over discovered projects plus active sessions; it is a
floating window; sessions can be renamed from it; it shows how many sessions are open.

S6. Stack: CLI with clap; config with serde + toml; XDG paths with directories; fast scanning that
respects `.gitignore` with ignore; `~` expansion with shellexpand; errors with thiserror + anyhow;
picker with ratatui + crossterm (and fzf as the alternative backend).

S7. Layouts are buildable from the interface itself, and resizable with the mouse or the keyboard.
Edits are saved, and session-scoped overrides disappear when the tmux session is closed or killed.
Each project has a default layout that the user can change from the interface. Global layouts are
also definable in the global config.

S8. Themes: reuse the color scheme of the `omarchy-control-center` project. The token set is the
one in `design/3 · Tokens y estados@1x.png`, verbatim:

```
fondo #14161c   superficie #1b1f28   selección #262c3a   borde #343b4d   texto #d9dde8
atenuado #98a0b4   acento #5ad1b5   ámbar #e8b45a   azul #7fa8ff   rosa #f08a9b
```

States: `●` acento = sesión activa; `◆` ámbar = sesión actual; `○` atenuado = sin sesión;
`×` rosa = error. Keys: `↑↓ mover`, `Enter abrir`, `Ctrl-X cerrar sesión`, `Ctrl-L cambiar layout`,
`Esc salir`. State is read from glyph and color together, never color alone.

S9. Clean Architecture, mirroring `omarchy-control-center` conventions: `domain` holds value types and
sync port traits; `application` holds use cases; `adapters` implement ports; `infrastructure` owns
process spawning, XDG paths, and logging; `ui` holds pure drawing types; `main.rs` is the only
`anyhow` boundary and the only place that wires dependencies.

S10. Logging with `tracing` to a file (never stderr, the TUI owns the terminal) with `EnvFilter`.
Tests: inline `#[cfg(test)] mod tests` per module with sentence-style names, integration tests in
`tests/`, and external effects faked behind a port — no test shells out to a real tmux.

S11. Decisions taken with the user (2026-11-09):

- D1 Phasing: núcleo primero, TUI después. Slice order: foundation/config/scan → tmux + open/list/
  last/kill/init → ratatui picker → layout editor → themes.
- D2 Picker: `builtin` (ratatui) por defecto, `fzf` opcional.
- D3 Al guardar un layout se pregunta el destino: `p` = `.tsess.toml` del proyecto, `g` =
  `config.toml` global, `s` = solo esta sesión (runtime, se purga al cerrar).
- D4 Temas: esquema de tokens del mockup 3 + las paletas builtin de `omarchy-control-center`, y temas
  de usuario en `~/.config/tsess/themes/*.toml` en `#rrggbb`.

## Tasks

| ID | Task | Specs | Route | Commit |
| --- | --- | --- | --- | --- |
| T1 | Foundation: scaffold, domain value types, XDG paths, config loading (global + project), project scanning with `ignore`, tracing to file, `list` and `init` subcommands | S2, S3, S6, S9, S10 | inline (writer guard) | `5258d45`, `469ceb8` |
| T2 | tmux adapter and session lifecycle: `open`, `last`, `kill`, layout materialization (windows/panes/send-keys), session runtime state + GC | S1, S4, S7, S9, S10 | worker | _pending_ |
| T3 | ratatui picker in a tmux floating popup with preview, rename and session counter; `fzf` alternative backend | S5, S6, S9, S10 | worker | _pending_ |
| T4 | Layout editor: build and resize panes with mouse and keyboard, save-destination prompt (`p`/`g`/`s`) | S7, S11 D3 | worker | _pending_ |
| T5 | Theme system: mockup token schema, control-center builtin palettes, user themes, theme picker | S8, S11 D4 | worker | _pending_ |

## Log

L1 (user, original request, verbatim):

> Quiero crear en rust la siguiente aplicación Una herramienta tipo tmux-sessionizer pero en Rust: escanea tus carpetas de proyectos, te muestra un fuzzy finder y crea o salta a una sesión de tmux con el layout que definas (NeoVim a la izquierda, servidor dev a la derecha, terminal abajo). Puedes añadir un archivo .sessionizer.toml por proyecto para definir paneles y comandos.
>
> Con los siguientes comandos tsess
>  # picker interactivo (caso principal)
> tsess open <ruta|nombre>   # abre o salta directo
> tsess list             # proyectos descubiertos + sesiones activas
> tsess last             # alterna con la sesión anterior
> tsess kill [nombre]    # cierra una sesión
> tsess init             # genera un .tsess.toml en el proyecto actual
>
> Con integración con tmux
>
> Con configuraciones globales. Que el usuario desde la propio interfaz pueda construir su propio layout, o tambien desde la configuración global crear sus propios layout y que cada proyecto tengo su layout por default y que el usuario pueda modificarlo por la propia interfaz a gusto
>
> Esto es un ejemplo de una configuración
>
> [search] roots = [ { path = "~/projects", depth = 2 }, { path = "~/study", depth = 1 }, ] markers = [".git", "package.json", "Cargo.toml"] exclude = ["node_modules", "target", "dist"]
> [picker] backend = "fzf"
> [defaults] layout = "dev"
> [layouts.dev] windows = [ { name = "editor", panes = [{ cmd = "nvim ." }] }, { name = "run", split = "vertical", panes = [{ cmd = "" }, { cmd = "" }] }, ]
>
> Para el proyecto usemos Clean Architecture
>
> Considero que estos serian algunos de los comandos de tmux involucrados, si hay mas o consideras que hace falta uno hazmelo saber
>
> tmux has-session -t =nombre / tmux new-session -d -s nombre -c /ruta -n editor / tmux send-keys -t nombre:editor "nvim ." Enter / tmux new-window -t nombre -n run -c /ruta / tmux split-window -h -t nombre:run -c /ruta / tmux switch-client -t =nombre / tmux attach-session -t =nombre
>
> CLI => clap / Config => serde + toml / Rutas XDG => directories / Escaneo rapido respetando .gitignore => ignore / Expandir => shellexpand / Errores => thiserror + anyhow / Picker => ratatui + crossterm + fzf
>
> Quiero que tenga selección temas tenemos un proyecto de omarchy-control-center puedes usar el esquema de colores de los temas creados. Quiero que se puedan crear layout desde la interfaz y tambien que se puedan redimencionar con el mouse o con el teclado. Que se guarde los layout y desaparezcan al momento de cerrar tmux o matar la session. Que el buscador de sesiones sea una ventana flotante, puedas renombrar la sesiones, y te muestre la cantidad de sesiones que tienes abiertas
>
> Implementas logging con tracing y testing
>
> Si hay alguna duda hazmelo saber

L2 (decisiones, 2026-11-09): respondidas las cuatro preguntas de alcance, picker, persistencia de
layouts y temas. Elegidas las cuatro opciones recomendadas → D1..D4 en S11. Además, por decisión del
orquestador: `.tsess.toml` es el nombre canónico (el mockup `3 · Tokens y estados` lo valida como
error de ejemplo) con `.sessionizer.toml` aceptado como fallback de lectura; los overrides de sesión
viven en `$XDG_STATE_HOME/tsess/sessions/<nombre>.toml` y se purgan comparando contra
`tmux list-sessions` al arrancar, no con hooks de tmux.

L3 (comandos tmux faltantes, informado al usuario): además de los siete comandos del pedido hacen
falta `display-message -p '#S'`, `list-sessions -F`, `set-option -t =n @tsess_layout`,
`new-window -d`, `split-window -d -p`, `send-keys … C-m`, `list-panes -F`, `resize-pane`,
`resize-window`, `select-layout`, `switch-client -l`, `rename-session`, `display-popup -E`,
`kill-session`. El `=` en los targets es obligatorio para no matchear por prefijo.

L4 (T1 cerrado, 2026-11-09): implementado inline porque el guard de writers exige la referencia
canónica `Spec: odd/tasks/<feature>.md, T<n>` y no se emitió. Evidencia: `cargo build`,
`cargo test --all-targets` (48 unit + 6 integración), `cargo clippy --all-targets -- -D warnings` y
`cargo fmt --check` en verde. El árbol de trabajo se dividió en dos work units: `5258d45`
(descubrimiento + `list`) y `469ceb8` (`init` + README + mockups). Los commits quedaron sin firma SSH
porque la clave de firma pide passphrase sin askpass en este entorno; re-firmar con
`git rebase --exec 'git commit --amend --no-edit -S' main`. `list` todavía no muestra sesiones
activas: eso entra con T2.
