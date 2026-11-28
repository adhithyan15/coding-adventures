---
category: Repo policy / workflow reminders
---

# Anchor patch paths to the active worktree before applying a correction

When several language worktrees share one task, copy the current absolute
worktree root into every patch path. A stale sibling path should fail before
any edit is applied; check the tool error, then retry only against the active
worktree.
