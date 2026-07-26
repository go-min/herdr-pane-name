# Contributing

## Development

Requirements:

- Rust 1.80 or newer;
- Herdr 0.7.5 or newer for live integration checks.

Run the same checks used by CI before opening a pull request:

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
git diff --check
```

Keep changes focused and preserve the plugin's cross-platform behavior. Do
not commit `target/`, local Herdr state, generated `labels.json`, or personal
configuration files.

Changes to the Herdr event manifest should be checked against the target
Herdr version's API schema. Unknown event names can leave a plugin installed
with warnings instead of failing loudly.
