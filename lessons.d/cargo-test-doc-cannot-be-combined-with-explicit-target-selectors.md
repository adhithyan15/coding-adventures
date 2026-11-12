---
category: Rust
---

# Cargo test --doc cannot be combined with explicit target selectors

`cargo test` rejects a command that combines `--doc` with explicit target
selectors such as `--lib` or `--test name`; it reports `Can't mix --doc with
other target selecting options` before compilation. Run the library and named
integration targets in one invocation, then run `cargo test -p <package> --doc`
as a separate invocation. Keep package `BUILD` files similarly split so the
documented local command matches CI.
