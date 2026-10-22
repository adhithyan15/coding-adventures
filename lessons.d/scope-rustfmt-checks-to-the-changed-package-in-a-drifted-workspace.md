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
