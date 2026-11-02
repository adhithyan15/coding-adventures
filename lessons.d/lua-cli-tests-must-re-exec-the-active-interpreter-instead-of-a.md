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
put `-e` chunks at `arg[-1]`. On Windows that entry may still be the configured
bare token `lua`, so request `lua.exe` explicitly rather than selecting a
same-named command wrapper. Replay the complete negative-index prefix, not only
the executable: dropping LuaRocks' `-e` loader chunk can make the child lose
native modules, silently take a shell fallback, and exercise different behavior
from the process under test.

Quoting the executable alone is also insufficient under `cmd /C`: when a
command starts with a quote and contains further quoted arguments or
redirections, `cmd` applies special first/last-quote stripping. Invoke the
complete child string through `cmd.exe /d /s /c` with the documented outer
quote pair, preserve inner argument quotes, and normalize Windows separators.
A real CLI test must prove the intended program ran, not merely that a
same-named PATH command exited successfully.
