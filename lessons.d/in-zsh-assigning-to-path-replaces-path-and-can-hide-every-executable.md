---
category: Repo policy / workflow reminders
---

# In zsh, assigning to path replaces PATH and can hide every executable

In zsh, `path` is the tied array form of the `PATH` environment variable. A
shell command used `path=/some/worktree` as an ordinary temporary variable;
the assignment replaced the executable search path, so every later `git`
invocation in that command failed with `command not found`.

Use task-specific names such as `worktree_dir`, `target_dir`, or
`checkout_path`. Treat zsh's `path` and common environment names as reserved,
especially in compound setup commands where the failure otherwise looks like
a missing tool rather than a variable collision.
