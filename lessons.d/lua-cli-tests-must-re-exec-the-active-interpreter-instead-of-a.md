---
category: Lua
---

# Lua CLI tests must re-exec the active interpreter instead of a PATH wrapper

A Windows Lua CLI regression test spawned the name `lua` from `PATH`. The CI
bootstrap exposed a shell wrapper under that name, so `cmd.exe` ran the wrapper's
Unix `mkdir -p` setup, returned success, and never executed the CLI being tested.

Re-exec the interpreter that is already running the test. Lua records the
interpreter and any launcher options at negative `arg` indexes; scan the
contiguous negative entries to the lowest index because LuaRocks launchers may
put `-e` chunks at `arg[-1]`. Quote that exact executable for the host shell and
normalize Windows separators. A real CLI test must prove the intended program
ran, not merely that a same-named PATH command exited successfully.
