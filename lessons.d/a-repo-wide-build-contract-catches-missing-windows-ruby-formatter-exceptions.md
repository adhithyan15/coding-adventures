---
category: Cross-platform & Windows BUILD_windows
---

# A repo-wide build contract catches missing Windows Ruby formatter exceptions

The repo-wide Windows BUILD audit rejected both Ruby microbenchmark packages because their `BUILD_windows` commands ran StandardRB without excluding `Layout/EndOfLine`. The repository enforces LF, while that cop expects native CRLF on Windows, so otherwise-correct checked-out files fail the Windows command.

Keep ordinary `BUILD` lint strict. In each Windows front, add only `--except Layout/EndOfLine`; do not convert files or suppress other cops. Run the actual metadata audit after editing both fronts, since it checks each command independently.
