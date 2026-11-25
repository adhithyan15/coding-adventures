---
category: Lua
---

# A Lua debug hook is per-thread: lua_newthread copies only the C trampoline, so a coroutine the code under test creates runs with no budget

The fuzz engine bounds a target with `debug.sethook(co, hook, "", 1000)` on the coroutine it
runs the target in. The first design said a coroutine the target creates "inherits the hook":
the C source copies `L->hook`, but that is only the C trampoline `hookf`, which looks the Lua
function up in a per-thread registry slot that the new thread does not have. A target that did
`coroutine.wrap(function() while true do end end)()` hung the whole test run, and the design
text had asserted the opposite.

Fix: for the duration of an execution replace `coroutine.create` and `coroutine.wrap` with
versions that call `debug.sethook(co, hook, "", n)` on what they make (the same closure, so the
budget is shared and a target cannot multiply it by spawning), then restore. A coroutine made
through a reference captured before that is still unbounded, and `while true do pcall(f) end`
cannot be stopped at all (a Lua hook cannot yield, and `pcall` catches the hook's error), so
say so in the spec and keep such targets out of the tests.

Test the claim, not the source: write the runaway-in-a-nested-coroutine target and run it under
an outer alarm before writing "inherits" or "bounded" in a spec. Found building VER06p
(`lua/test_bootstrap`).
