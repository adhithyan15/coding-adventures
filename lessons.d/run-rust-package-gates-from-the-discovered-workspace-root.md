---
category: Repo policy / workflow reminders
---

# Run Rust package gates from the discovered workspace root

The first focused Cargo test ran from `code/`, which has no workspace manifest,
so Cargo stopped before compiling the intended red test. Confirm the workspace
manifest path before running package gates; for repository Rust packages the
current workspace root is `code/packages/rust`.
