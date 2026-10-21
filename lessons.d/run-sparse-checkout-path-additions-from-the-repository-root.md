---
category: Repo policy / workflow reminders
---

# Run sparse-checkout path additions from the repository root

`git sparse-checkout add code/packages/...` was invoked from a nested program
directory, so the pathspecs were resolved relative to that directory and the
required root-relative paths stayed hidden without an error. Run sparse-checkout
updates from the worktree root, then verify both `git sparse-checkout list` and
the expected directory before retrying the command that needed it.
