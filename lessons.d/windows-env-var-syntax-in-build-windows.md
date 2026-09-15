---
category: Cross-platform & Windows BUILD_windows
---

# Windows env-var syntax in BUILD_windows

Use `set "VAR=value" && command` (defensive quoting handles `&|()` in paths/`%CD%`), NOT Unix-style `VAR=value command`. `if [ -f ]`, `elif`, `fi` all break — translate to CMD or skip on Windows.
