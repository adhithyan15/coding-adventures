---
category: Rust
---

# Scope rustfmt checks to the changed package in a drifted workspace

Running `cargo fmt --all -- --check` for one new X.509 package traversed the
entire monorepo workspace and stopped on extensive pre-existing formatting
drift in unrelated compiler and UI crates, so it never reached the focused
tests. In a large workspace with known unrelated drift, run rustfmt through the
changed package manifest (and still inspect the complete Git diff) rather than
using `--all`; reserve a workspace-wide check for an explicitly scoped cleanup.

This recurred even with `--manifest-path`: adding `--all` still selected the
enclosing workspace and emitted megabytes of unrelated diffs. For a focused
check, run `cargo fmt --manifest-path <package>/Cargo.toml -- --check` with no
`--all`, or invoke `rustfmt --check` on the exact changed Rust source.
