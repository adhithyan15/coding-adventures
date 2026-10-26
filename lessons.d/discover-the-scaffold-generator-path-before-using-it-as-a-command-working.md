---
category: Repo policy / workflow reminders
---

# Discover the scaffold generator path before using it as a command working directory

The repository lesson named the canonical Go scaffold-generator directory, but
the current sparse worktree did not materialize that tracked path. Setting the
command working directory therefore failed before `ls` or `go run` could
explain the problem. Resolve the live path with `git ls-tree`, inspect
`git sparse-checkout list`, and add the tracked generator directory from the
worktree root before using it as a command working directory.
