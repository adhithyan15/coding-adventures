---
category: BUILD files & dependency management
---

# Each BUILD line runs as a separate `sh -c` (Unix) / `cmd /C` (Windows) process

`cd` and shell variables do NOT persist between lines. Chain with `&&` on one line, use subshells `(cd ../dep && ...)`, or keep each line absolute. Multiline `if/then/fi`, `for`, and backslash continuations all break — the runner sees `\` as a literal command and fails with `\: not found`.
