# Changelog

## 0.2.0 - 2026-10-07

- require Herdr 0.9.3+ and synchronize from one session snapshot;
- preserve pane ownership across moves through stable terminal IDs;
- prefix agent display names without changing semantic identity or state;
- restore previous agent display names on clear/reset and preserve manual edits;
- add opt-in OSC title naming with `terminal_titles = true`;
- reconcile current ownership before clearing labels;
- accept empty successful CLI output from metadata mutations;
- serialize concurrent event and shell hooks with a cross-platform file lock.

## 0.1.0 - 2026-07-27

Initial public release:

- automatic workspace, tab, and pane naming from foreground processes;
- built-in Nerd Font icons for common tools;
- manual-label preservation with automatic numeric prefixes;
- zsh, Bash, and fish hooks;
- `reset` and `clear` actions;
- cross-platform Rust implementation and CI.
