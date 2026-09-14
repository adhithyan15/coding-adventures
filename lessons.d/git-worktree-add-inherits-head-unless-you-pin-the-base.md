---
category: Cross-platform & Windows BUILD_windows
---

# `git worktree add` inherits HEAD unless you pin the base

Always `git worktree add <path> -b <branch> origin/main`. Whenever the source checkout is shared or noisy, default to a fresh worktree from `origin/main` to avoid accidentally committing other agents' files or shared-manifest pollution.
