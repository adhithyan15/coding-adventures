---
category: Rust
---

# A Rust crate rename requires refreshing every separate transitive Cargo lockfile

The heap package was renamed from `heap` to `coding-adventures-heap`. The main
Rust workspace lock and direct wrapper lockfiles were updated, but the
standalone WASM and Rust program roots still recorded the old package name
through transitive dependencies. A Windows `cargo test --locked` job failed
when it tried to update `wasm/mini-redis/Cargo.lock`.

Inspect every tracked lockfile containing the old package entry and resolve
the manifest graph for each root that reaches the renamed crate. Refresh those
locks with Cargo (`cargo tree` or `cargo update` from each root), then run its
`--locked` build or test. The workspace reverse-dependency graph does not
include separately rooted consumer packages.
