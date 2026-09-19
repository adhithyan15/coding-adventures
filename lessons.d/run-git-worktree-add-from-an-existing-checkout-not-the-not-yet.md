---
category: Repo policy / workflow reminders
---

# Run git worktree add from an existing checkout, not the not-yet-created target directory

An execution wrapper resolves its working directory before it starts the shell,
so a command cannot create the very directory selected as that command's cwd.
Run `git worktree add` from an existing checkout, then switch subsequent
commands to the newly created worktree only after the add succeeds.
