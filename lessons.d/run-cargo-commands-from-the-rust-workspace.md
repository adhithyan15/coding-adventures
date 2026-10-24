---
category: Repo policy / workflow reminders
---

# Run Cargo commands from the Rust workspace

A combined validation command invoked `cargo tree` from the repository root,
where no `Cargo.toml` exists, so the first gate stopped before later checks
ran. Run Cargo commands from `code/packages/rust` (or pass an explicit
manifest path), and run repository-level scripts from the worktree root.
