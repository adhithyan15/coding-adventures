---
category: Repo policy / workflow reminders
---

# Point lesson commands at the checked-out shard directory

`lessons.py new` defaults to a root-level `lessons.d`, but that tracked
directory was outside the inherited sparse-checkout set, so the command could
not read `_meta.md`. Before using the lesson CLI in a sparse worktree, confirm
the shard directory is checked out and add that exact tracked path when it is
missing.
