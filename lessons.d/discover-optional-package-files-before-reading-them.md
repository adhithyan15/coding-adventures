---
category: Repo policy / workflow reminders
---

# Discover optional package files before reading them

Package conventions are not universal: a crate may have `BUILD` without
`BUILD_windows`, or may omit another otherwise common artifact. Reading a
guessed optional path turns a harmless absence into a failed command and can
skip later checks chained after it.

List the package directory or use `rg --files <package>` first, then read and
update only files that actually exist. Keep independent status checks in a
separate command so one optional-file miss cannot suppress them.
