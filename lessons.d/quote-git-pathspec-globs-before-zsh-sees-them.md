---
category: Repo policy / workflow reminders
---

# Quote Git pathspec globs before zsh sees them

An unquoted wildcard intended for `git grep` is expanded by zsh first. When the
current checkout has no matching path, the shell aborts with `no matches found`
and Git never receives the pathspec.

Quote wildcard pathspecs passed to Git, including exclusions. This preserves
Git's own tree-aware matching and keeps sparse-checkout shape from changing the
meaning of an audit command.
