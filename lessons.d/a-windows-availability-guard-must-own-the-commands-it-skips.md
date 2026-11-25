---
category: Cross-platform & Windows BUILD_windows
---

# A Windows availability guard must own the commands it skips

The build tool starts every `BUILD_windows` line as a separate `cmd /C`
process. A line that exits successfully after printing “tool unavailable —
skipping” does not stop later lines from running. If those later lines require
the missing tool, the package still fails.

Put the availability check and all commands it guards in one batch script (or
one command line). Return success only from the unavailable branch; when the
tool exists, propagate every test and consumer failure. Validate both branches
with a missing-tool PATH and a stub tool that exits nonzero.
