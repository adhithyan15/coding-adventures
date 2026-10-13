---
category: Repo policy / workflow reminders
---

# Only run git worktree unlock after a read-only check confirms that the worktree is actually locked.

A newly created worktree was already usable and unlocked, but setup ran `git worktree unlock` unconditionally. Git correctly returned a fatal error because there was no lock to remove. Inspect `git worktree list --porcelain` or the worktree lock file first, and unlock only when a lock is present; ordinary `git worktree add` needs no unlock step.
