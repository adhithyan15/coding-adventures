---
category: Repo policy / workflow reminders
---

# Run repository-root git paths from the repository root

A combined validation command ran from a package subdirectory but tried to
stage repository-root-relative paths, so Git could not resolve them. The first
attempt to record the failure also guessed an unsupported lesson category.
Run repository-wide Git operations from the worktree root, run package commands
from their package directory separately, and choose a category from the
generator's reported allowlist.
