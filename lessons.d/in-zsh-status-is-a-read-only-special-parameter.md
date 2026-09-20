---
category: Repo policy / workflow reminders
---

# In zsh, status is a read-only special parameter

In zsh, `status` is the read-only special parameter that exposes the previous
command's exit status. Assigning `status=$?` aborts the command with a
`read-only variable` error, which can hide output or prevent later cleanup and
reporting steps from running.

Use a task-specific name such as `command_rc`, `test_rc`, or `coverage_rc`
when capturing an exit code in commands that may run under zsh.
