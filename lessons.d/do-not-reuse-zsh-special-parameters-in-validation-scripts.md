---
category: Repo policy / workflow reminders
---

# Do not reuse zsh special parameters in validation scripts

In zsh, `status` is a read-only special parameter containing the last command's
exit status, while `path` is the tied array form of `PATH`. Assigning probe
output to `status` aborts the script; using `path` as a loop variable can erase
the executable search path mid-command. Use task-specific names such as
`probe_status` and `candidate_path`, and avoid shell-special parameter names
when collecting or iterating over validation state.
