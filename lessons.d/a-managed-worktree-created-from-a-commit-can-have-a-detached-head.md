---
category: Repo policy / workflow reminders
---

# A managed worktree created from a commit can have a detached HEAD

The managed worktree was created from a pinned commit. Its checkout had a
detached HEAD, so `git branch -m` failed before any implementation edit.
`git switch -c codex/parity-haskell-registry-20261012` created the intended
branch without moving or resetting files. Check `git symbolic-ref --quiet
--short HEAD` after creation: rename an existing fresh branch, or create a
new branch for a detached checkout. Verify the exact base and clean tracked
state before carrying the parity journal into that worktree.
