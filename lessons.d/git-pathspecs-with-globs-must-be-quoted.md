---
category: Repo policy / workflow reminders
---

# Git pathspecs with globs must be quoted

An unquoted wildcard intended for `git grep` was expanded by zsh before Git
could interpret it, and the command failed when the checkout had no matching
filesystem path. Quote wildcard pathspecs so Git, not the shell, owns their
matching semantics.
