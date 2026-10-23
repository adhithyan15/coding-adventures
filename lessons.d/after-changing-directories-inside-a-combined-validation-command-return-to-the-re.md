---
category: Repo policy / workflow reminders
---

# After changing directories inside a combined validation command, return to the repository root explicitly before invoking repository-relative scripts.

A combined validation command changed from `code/packages/rust` to `code` and
then invoked `code/scripts/lessons.py`, producing a duplicated `code/code`
path. The Rust gates had completed, but the final lesson validator had not run.
Use a subshell for package-local commands or `cd` back to the recorded
worktree root before repository-relative checks; never infer the final working
directory from a chain of relative `cd` operations.
