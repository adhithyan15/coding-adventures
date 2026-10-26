---
category: Repo policy / workflow reminders
---

# Do not assign to zsh's lowercase `path` parameter in shell automation

In zsh, `path` is a special array tied to `PATH`. Assigning a worktree pathname
to a variable named `path` replaces the command-search path, so the very next
`git` invocation fails with `command not found` even though Git is installed.
Use a task-specific name such as `worktree_dir` for filesystem paths in shell
automation, especially in commands that continue by invoking other programs.
