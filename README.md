# herdr-pane-name

[![CI](https://github.com/ter-sh/herdr-pane-name/actions/workflows/ci.yml/badge.svg)](https://github.com/ter-sh/herdr-pane-name/actions/workflows/ci.yml)

Cross-platform Herdr plugin that derives tab and pane names from the
foreground process. It preserves labels changed by the user, adds `1:`–`9:`
jump-key prefixes, and supports zsh, Bash, and fish hooks.

## Requirements

- Herdr 0.7.5 or newer
- Rust 1.80+ to build from source

The plugin uses only the Herdr CLI exposed through `HERDR_BIN_PATH`; it does
not require Bash, `jq`, or a platform-specific socket implementation.

## Install

Install the published GitHub plugin directly through Herdr:

```sh
herdr plugin install ter-sh/herdr-pane-name -y
```

The plugin builds itself from the manifest, registers its startup and
lifecycle hooks, and persists across Herdr restarts. Verify the installation:

```sh
herdr plugin list
herdr plugin config-dir herdr.pane-name
```

The built-in Herdr events handle workspace/tab/pane lifecycle and focus
changes. For the fastest updates immediately after arbitrary shell commands,
load the optional shell hook from a local checkout as described below. A
dedicated Herdr foreground-process event would remove that last integration
step; see [API boundary](#api-boundary).

## Development build and link

```sh
cargo test
cargo build --release
herdr plugin link .
```

The manifest contains event hooks for workspace, tab, pane, and agent
lifecycle changes plus a startup sync. Herdr's event hook is a short-lived
process, so the plugin does not hold a raw socket open or require a daemon.
For command-level process changes between Herdr lifecycle events, optionally
load one shell hook:

```sh
export HERDR_PANE_NAME_BIN="$PWD/target/release/herdr-pane-name"
source hooks/zsh.zsh       # zsh
source hooks/bash.bash     # Bash
source hooks/fish.fish     # fish
```

The hooks run after each command and shortly after a command starts. When
`HERDR_PANE_NAME_BIN` is not set and the binary is not on `PATH`, they call the
installed Herdr plugin action directly, so a GitHub-installed plugin does not
need a second binary installation. All Herdr communication and JSON handling
is implemented by the Rust binary.

## Configuration

Copy `config.example.toml` to the directory printed by
`herdr plugin config-dir herdr.pane-name`. The plugin reloads it on every
event, so editing it does not require a restart.

The shell hooks discover this directory automatically. Set
`HERDR_PLUGIN_CONFIG_DIR` yourself only when you need to override it. Set
`HERDR_PANE_NAME_BIN` to an absolute or repository-relative path when the
binary is not available as `herdr-pane-name` in `PATH`.

```toml
max_length = 32
show_args = true
icons = true
prefixes = true
ignored_programs = ["ssh", "top"]
```

`max_length` includes the numeric prefix. `show_args` appends up to three
command arguments. Icons are enabled by default. When `icons = true`, the
plugin prepends a built-in Nerd Font glyph to the process name, for example
`1: nvim`; it never replaces the process name. The catalog covers common
shells, editors, languages, package managers, containers, cloud tools,
databases, and terminal utilities. The glyph mapping is built into the
plugin; unknown programs keep their normal process name.

Examples from the built-in catalog include `hunk` (` hunk`), Antigravity's
`agy` (`󰚩 agy`), and Herdr itself (` herdr`).

Tab positions are counted independently inside each workspace, and pane
positions independently inside each tab. Only positions 1–9 receive a
numeric prefix. Workspace names keep Herdr's default base name and receive
only their workspace prefix.

For example, a manually renamed tab `1:review` remains `review` as its base
name. If it becomes the second tab, the plugin changes it to `2:review`; it
does not replace `review` with the foreground process. Manually assigned
names without a plugin-owned prefix are left unchanged.

`reset` removes only labels owned by this plugin and forgets ownership.
`clear` removes plugin-owned labels but keeps the ownership state, so the next
automatic update can restore them. Manual labels are never touched.

The plugin stores ownership state in `labels.json` under Herdr's plugin config
directory. It does not read or write the repository checkout after install.

Agent semantic names are intentionally left unchanged. Herdr currently
rejects the requested `1:agent` form in `agent rename`; agent prefixes need a
separate display-label API in Herdr. Agent lifecycle events are still useful
because they trigger a tab/pane resynchronization.

```sh
herdr plugin action invoke herdr.pane-name.reset
herdr plugin action invoke herdr.pane-name.clear
```

## API boundary

Herdr currently exposes everything needed to read a foreground process and
rename an individual pane/tab. It does not expose a first-class “automatic
label” or “manual label” bit, nor a plugin-owned long-lived event subscription.
This plugin compensates with a small `labels.json` ownership state file in the
plugin config directory and manifest event hooks. A future minimal Herdr API
addition would be `label_source` on pane/tab records plus a plugin-owned rename
operation; that would remove the state-file heuristic. A native subscription
hook would also reduce process spawning, but is not required for correctness.

Herdr's current plugin event surface does not provide a dedicated
`pane.foreground_process_changed` event with process/argv payload. The plugin
therefore reads `pane process-info` during lifecycle events and uses the
optional shell hooks for command-level updates. This is the only known reason
the install cannot yet be completely shell-integration-free.

## License

MIT. See [LICENSE](LICENSE).
