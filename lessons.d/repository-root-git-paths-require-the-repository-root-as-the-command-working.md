---
category: Repo policy / workflow reminders
---

# Repository-root Git paths require the repository root as the command working directory

Running `git add lessons.d/...` from `code/packages/rust` resolves that path
under the Rust workspace and fails before a chained test command begins. Use
the repository root for Git paths that begin with `lessons.d/`, `code/specs/`,
or `code/packages/`. Run package-local Cargo commands separately with the Rust
workspace as their working directory so one command cannot silently prevent
the other workflow stage.
