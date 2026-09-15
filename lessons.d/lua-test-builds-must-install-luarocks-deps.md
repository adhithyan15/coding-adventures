---
category: Lua
---

# Lua test BUILDs must install LuaRocks deps

declared in the rockspec (`luasocket`, etc.) before invoking busted. Native deps may fail to compile on Windows; gate with `BUILD_windows` no-op.
