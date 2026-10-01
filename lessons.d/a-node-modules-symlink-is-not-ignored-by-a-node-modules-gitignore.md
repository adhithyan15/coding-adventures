---
category: Repo policy / workflow reminders
---

# A node_modules symlink is not ignored by a node_modules/ gitignore pattern

**What happened:** to save disk, extra git worktrees share one installed
dependency tree: each worktree's
`code/packages/typescript/<pkg>/node_modules` is a symlink to the real
directory in one host worktree. Committing a Russian tranche, I staged
with `git add -A code/packages/typescript/human-language-data`, and the commit
picked up `human-language-data/node_modules` as a new file: a symlink
pointing into my scratch directory.

The repository ignores `node_modules/`. A pattern with a trailing slash
matches only directories, and git records a symlink as a file, so the pattern
does not apply to it. `git status` lists the symlink as untracked (`??`),
which is easy to miss in a long status.

I caught it before pushing by reading the staged file list. The fix was
`git rm --cached <path>/node_modules` and an amend. The symlink stays on disk.

**What to do differently:**

1. When `node_modules` is a symlink, stage only the paths you mean to commit
   (`code/learning`, the package's `tests/`, `CHANGELOG.d/`). Never run
   `git add -A` on a whole package directory.
2. Before committing, check the staged list:
   `git diff --cached --name-only | grep node_modules` must print nothing.
3. Alternatively, add the symlink paths to `.git/info/exclude` in each
   worktree. That file is local and never committed.
