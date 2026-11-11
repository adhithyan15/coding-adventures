---
category: Repo policy / workflow reminders
---

# Do not use path as a zsh loop variable

A zsh cleanup loop named its iterator `path`. In zsh, `path` is a special array
tied to the `PATH` environment variable, so assigning a repository filename to
it removed command lookup and every subsequent `git` invocation failed before
changing files.

Use neutral iterator names such as `file_path` in portable shell loops. When a
loop unexpectedly loses basic commands, inspect shell-special variables before
changing files or deleting lock files; rerun only after proving that no Git
operation was started.
