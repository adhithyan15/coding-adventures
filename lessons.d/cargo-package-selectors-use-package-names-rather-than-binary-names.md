---
category: Rust
---

# Cargo package selectors use package names rather than binary names

The CV01 import-repair validation first used `cargo test -p closurec`. Cargo
rejected that selector because `closurec` is the binary name; its package is
`coding-adventures-closurec`. This was an invocation error, not a compiler or
test failure. The corrected full suite runs `cargo test` from the program
directory, followed by `cargo clippy --all-targets -- -D warnings` there.

Read `[package].name` in Cargo.toml before using `-p`. Binary selection uses
`--bin`; it answers a different question and does not identify the package.
For shared crates, use their own `--manifest-path` so development dependencies
and test targets resolve in the intended package context. Preserve the exact
successful command with the validation evidence rather than repeating a
remembered selector derived from the executable filename.
