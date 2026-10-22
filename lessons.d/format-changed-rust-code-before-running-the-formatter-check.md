---
category: Rust
---

# Format changed Rust code before running the formatter check

The first post-implementation `cargo fmt --check` found a routine line-wrap
difference and prevented the chained focused test from running. After editing
Rust, run package-scoped `cargo fmt` first, then use `--check` as the clean-tree
verification gate before tests and review.
