---
category: Rust
---

# A sparse Rust worktree must materialise every workspace member's manifest and target entry files before cargo builds any crate

A sparse checkout of only the crates under test (`python-to-semantic-ir`,
the SIR backends and their path dependencies) could not run any `cargo`
command. `code/packages/rust/Cargo.toml` is one workspace with hundreds of
members, and cargo loads *every* member's manifest before building one crate.
It failed in three stages:

1. "failed to load manifest for workspace member" when a member's
   `Cargo.toml` was absent.
2. "no targets specified in the manifest" when its `src/lib.rs` /
   `src/main.rs` was absent.
3. "can't find `<name>` test at `tests/<name>.rs`" for a member that declares
   an explicit `[[test]]` / `[[bench]]` / `[[example]]` / `[[bin]]` target.

What works: use non-cone patterns.

- Add the full directories of the crates you build, and their transitive
  path dependencies.
- Add `/code/packages/rust/**/Cargo.toml`, `/code/packages/rust/**/src/lib.rs`
  and `/code/packages/rust/**/src/main.rs` for every other member.
- Add each path that a `tomllib` scan of the manifests finds in
  `[[test]]`/`[[bench]]`/`[[example]]`/`[[bin]]`/`[lib] path`/`build`.

Also add any non-Rust inputs that a needed crate's `build.rs` or tests read:

- `code/grammars/<lang>/` for the grammar-driven lexers and parsers;
- `code/packages/python/sir-runtime-*/src/` for the Python backend's e2e
  runs, which otherwise skip or fail.

The whole set is about 80 MB, against 210 MB for the full
`code/packages/rust` tree. Use `cargo metadata --no-deps --offline` as a
cheap probe after each sparse addition.
