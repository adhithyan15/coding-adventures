---
category: Rust
---

# Scope rustfmt to touched packages in large workspaces

Running `cargo fmt --all -- --check` in this monorepo reports pre-existing
formatting differences across unrelated crates and can emit millions of bytes,
obscuring the files under review.

For a bounded package change, run `cargo fmt -p <package> -- --check` for each
touched package, or invoke `rustfmt` on the exact files when package scoping is
unavailable. Reserve a workspace-wide check for an explicit repository-wide
formatting task.
