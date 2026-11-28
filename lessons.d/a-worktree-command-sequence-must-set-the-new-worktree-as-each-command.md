---
category: Repo policy / workflow reminders
---

# A worktree command sequence must set the new worktree as each command's working directory

Creating a worktree does not change the current shell's working directory. In a
multi-command shell call, a `git worktree add ...` followed by `git
sparse-checkout init` or `git sparse-checkout set` still runs those latter
commands in the original checkout unless the command explicitly changes
directory. That can alter the original checkout's sparse patterns and make
tracked files appear deleted.

For every command that operates on a worktree, set its `workdir` to that exact
worktree path (or use an explicit `git -C <worktree>`). After a worktree is
created, verify `git -C <worktree> rev-parse --show-toplevel` before running any
setup command. Never assume a worktree creation command updates the caller's
working directory.
