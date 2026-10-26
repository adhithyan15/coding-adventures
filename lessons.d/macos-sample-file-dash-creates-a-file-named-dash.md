---
category: Repo policy / workflow reminders
---

# macOS sample -file dash creates a file named dash

Unlike tools that treat `-` as standard output, macOS `sample -file -` writes
its report to a literal file named `-` in the current directory. That can
quietly appear as an untracked repository file during diagnostics. Use an
explicit path under `/tmp` for `sample` output, inspect it there, and remove
the temporary report after use.
