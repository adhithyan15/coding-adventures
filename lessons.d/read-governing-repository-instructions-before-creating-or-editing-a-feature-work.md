---
category: Repo policy / workflow reminders
---

# Read governing repository instructions before creating or editing a feature worktree

The intermediate-CA policy slice created its sibling worktree and updated two
lesson shards before reading that worktree's `AGENTS.md` and `CLAUDE.md`. The
base was correct and the edits were recoverable, but that ordering could have
missed repository-local constraints governing the branch location, required
preflight reading, or permitted changes.

Before `git worktree add`, inspect the applicable instruction files from the
exact fetched base with `git show`. After checkout, read them again from the
managed worktree before the first edit. Treat worktree creation and lesson
updates as task actions, not as instruction-free setup.
