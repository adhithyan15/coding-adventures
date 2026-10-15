---
category: Cross-platform & Windows BUILD_windows
---

# Prefer saved Python helpers over nested PowerShell inline quoting

A nested python -c generator used backslash-escaped quotes through PowerShell.
PowerShell changed those arguments, and Python rejected the generator before
writing or running the current-main build plan. Save the short Python helper
with a structured patch and execute its path instead. Preserve the failure and
do not count the later actual plan until its own command terminates successfully.
This lesson is recorded in the isolated next-slice worktree so the immutable
reviewed publication candidate remains untouched.
