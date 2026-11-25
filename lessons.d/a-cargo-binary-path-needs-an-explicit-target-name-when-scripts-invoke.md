---
category: Rust
---

# A Cargo binary path needs an explicit target name when scripts invoke a named binary

# A Cargo binary path needs an explicit target name when scripts invoke a named binary

When a package uses a custom `src/bin/main.rs` path, Cargo infers the target
name as `main`. If BUILD files, README commands, or operators invoke a stable
binary name, declare an explicit `[[bin]]` target with `name` and `path` in
`Cargo.toml`, then exercise that exact name in the acceptance command.

This surfaced when a native acceptance run asked Cargo for
`starlark-microbenchmark` and the manifest exposed only `main`.
