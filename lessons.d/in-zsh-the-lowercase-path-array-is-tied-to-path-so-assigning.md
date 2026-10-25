---
category: Repo policy / workflow reminders
---

# In zsh the lowercase path array is tied to PATH, so assigning a scalar path variable can hide every command

In zsh, `path` is a special array tied directly to the `PATH` environment
variable. A shell loop that assigned a scalar filename to `path` accidentally
replaced the command search path, so later invocations of `rg` and `head`
failed as if the programs were not installed. Use a task-specific name such
as `lesson_file` for local shell variables and reserve both `PATH` and `path`
for the shell's command lookup state.
