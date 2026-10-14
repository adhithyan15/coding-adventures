---
category: Repo policy / workflow reminders
---

# Wait for an initializing worktree process before inspecting or removing it

A large `git worktree add` outlived the command tool's first output yield. A
second status command ran during checkout and printed millions of apparent
deletions, and an attempted forced cleanup then failed because Git had correctly
locked the worktree as `initializing`. Before diagnosing a new worktree as
corrupt, inspect `git worktree list --porcelain` and the live process table. If
the checkout process is still active, wait for it; only inspect or remove the
worktree after the initialization lock clears.
