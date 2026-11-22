---
category: Rust
---

# Cargo fmt with a file argument may still format an entire Rust workspace

Running `cargo fmt -- path/to/file.rs` from a Cargo workspace formatted thousands of Rust files in
the workspace in this environment. The focused test was unrelated to that change, and the command
completed before the mistake was noticed. Before formatting one file, invoke `rustfmt --edition
2021 /absolute/path/to/file.rs` directly. If a workspace-wide formatter is ever intended, inspect
the exact repository instructions first and review the complete diff immediately afterward.

For recovery, snapshot the full diff outside the worktree, restore only paths proven clean before
the formatter ran, and preserve all deliberate concurrent edits. Never use a broad reset or clean
to undo a formatting accident.
