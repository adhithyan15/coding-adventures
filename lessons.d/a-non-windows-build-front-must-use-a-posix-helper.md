---
category: Cross-platform & Windows BUILD_windows
---

# A non-Windows BUILD front must use a POSIX helper

The build tool selects `BUILD` on Unix-like hosts and `BUILD_windows` on
Windows, then executes each listed command through that host's shell. A
PowerShell command copied into `BUILD` fails with `powershell: not found` on
Linux even when the preceding build succeeds. Put Unix-like checks in a POSIX
shell helper invoked by `BUILD`; keep the equivalent PowerShell helper in
`BUILD_windows`. Validate the actual installed consumer and CLI from both
fronts so their assertions do not drift.
