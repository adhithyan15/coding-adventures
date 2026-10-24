---
category: Cross-platform & Windows BUILD_windows
---

# Group Windows setup conditionals before chaining build commands

In a cmd.exe recipe, an ungrouped `if not exist ... mkdir ...` can capture the
following `&&` chain as its conditional body. When the directory already
exists, the command can exit successfully without invoking the native build.
Parenthesize each setup conditional before chaining later commands. Validate
the front twice with directories already present and require actual test
counts and coverage output; exit code zero alone does not prove execution.
