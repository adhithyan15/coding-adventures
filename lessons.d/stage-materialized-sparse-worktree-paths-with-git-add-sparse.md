---
category: Repo policy / workflow reminders
---

# Stage materialized sparse-worktree paths with git add --sparse

A file can be present and editable in a sparse worktree while its index path
remains outside the sparse definition. Plain `git add <path>` then refuses to
update it, even though source edits and tests succeeded. For an already
prepared worktree, stage the intended path with `git add --sparse`; do not
reinitialize or expand another checkout's sparse rules as a workaround.
