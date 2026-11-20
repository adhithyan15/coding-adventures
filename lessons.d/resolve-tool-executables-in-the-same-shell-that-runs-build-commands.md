category: Cross-platform & Windows BUILD_windows
---

# Resolve tool executables in the same shell that runs BUILD commands

A PowerShell session can resolve a shim or executable through its own command
resolution while a child `cmd.exe` launched by a Python harness has a different
`PATH`. That makes a BUILD front fail before the package command starts. For
cross-platform validation, invoke the verified executable path directly or
construct and probe the exact child environment before running the front. Keep
the process-scoped adjustment out of global settings.
