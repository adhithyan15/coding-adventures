---
category: Lua
---

# luacov attributes a line to the frame two levels above its hook, so chaining it under another debug hook loses the coverage it was measuring

A test helper that bounds runaway code with `debug.sethook(fn, "", N)` and forwards
`line` events to the previously installed hook (luacov's) adds one stack level, so
luacov credits every line to the helper instead of the code under test: coverage fell
from 94% to 34% with all tests green. Skip the bound when measuring
(`if os.getenv("LUA_COVERAGE") then return fn(...) end`), and restore a previous hook
with all three of `debug.gethook`'s results (`debug.sethook(f, mask, count)`); passing
only `f` raises `bad argument #2 to 'sethook'`.

Found building the VER05o tests (`lua/test_bootstrap`).
