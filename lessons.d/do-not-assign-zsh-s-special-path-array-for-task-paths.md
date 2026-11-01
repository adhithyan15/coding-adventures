---
category: Repo policy / workflow reminders
---

# Do not assign zsh's special path array for task paths

In zsh, `path` is a special array tied to the executable-search `PATH` value.
Assigning a worktree path to a variable named `path` replaced the command
search path for that shell invocation, so even `git` appeared unavailable.
No repository state changed, but the diagnostic was misleading. Use a
task-specific name such as `worktree_path` or `wt_path`, and reserve shell
special names for their intended meaning. The same rule applies to zsh's
read-only `status` parameter: capture an exit code in a task-specific variable
such as `command_exit` instead of assigning to `status`.
