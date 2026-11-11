---
category: Rust
---

# Cargo file arguments are relative to the active workspace directory

An X.509 package check ran from `code/packages/rust` but passed repository-root
paths after Cargo's `--` separator. `cargo fmt` interpreted those paths relative
to the active Rust workspace and reported that both files did not exist.

When invoking Cargo from a nested workspace, make every explicit file argument
relative to that workspace (for example, `x509-package/src/lib.rs`), or run the
formatter from the repository root with repository-relative paths. Check the
current working directory before copying paths from a repository-root diff.
