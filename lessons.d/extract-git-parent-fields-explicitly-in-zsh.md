---
category: Repo policy / workflow reminders
---

# Extract Git parent fields explicitly in zsh

In zsh, `set -- $scalar` does not perform the implicit whitespace splitting
that a POSIX-shell verification snippet may expect. A squash-merge check used
that form on `git rev-list --parents`, left `$2` empty, and passed an empty
parent to `git merge-tree`. Extract the first parent with `git rev-parse
"$merge^1"` and count fields separately with `awk`; do not make merge-tree
verification depend on shell-specific scalar splitting.
