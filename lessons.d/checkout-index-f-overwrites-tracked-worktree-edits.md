---
category: Repo policy / workflow reminders
---

# `checkout-index -f` overwrites tracked worktree edits

After changing sparse-checkout patterns, `git checkout-index -a -f` restores
every tracked path from the index and discards uncommitted edits, including
edits another worker made in the shared worktree. Materialize only the intended
paths before editing, and do not use a forced checkout-index across a dirty
worktree. Check status and commit or preserve concurrent edits before any
index-based checkout operation.
