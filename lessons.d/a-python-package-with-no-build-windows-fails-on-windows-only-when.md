---
category: Cross-platform & Windows BUILD_windows
---

# A Python package with no BUILD_windows fails on Windows only when a dependency change first rebuilds it there

The dependency-pinning PR (#19221) converted Python BUILD_windows files from
plain `pip` to `uv pip`. Changing `simulator-protocol`'s and `logic-gates`'
BUILD_windows made the Windows lane rebuild every dependent, and 19 of them
had **no** BUILD_windows: 12 `*-simulator` packages, then (one wave later,
once those had one) 7 `*-gatelevel` packages. Windows ran their POSIX
`BUILD` under cmd and failed with `'.venv' is not recognized as an internal
or external command`, because Windows virtualenvs keep executables in
`.venv\Scripts`, not `.venv/bin`.

`audit_build_windows.py` has a check for exactly this (class E,
windows-posix-fallback), but its POSIX patterns did not include `.venv/bin/`,
so all 19 sat silently until something rebuilt them.

Fix: every Python package now has a BUILD_windows, and `.venv/bin/` is a
POSIX construct in the audit, so a new Python package without one blocks
the contracts job immediately. `programs/python/conduit-hello` is the one
known (advisory) exception.

What to do differently: when a change rebuilds a dependency *hub* on
Windows, first list its dependents with no BUILD_windows. They fail in
waves, one level of the graph per CI run, so fixing only the failures you
see costs one Windows run (an hour or more of queue) per level.
