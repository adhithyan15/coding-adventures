---
category: Native extensions & FFI
---

# Lua 5.4 `LUA_REGISTRYINDEX = -1_001_000`

(derived from `-LUAI_MAXSTACK - 1000`), NOT the Lua 5.1 value `-10000`. Using `-10000` in `luaL_ref` treats it as a regular negative stack index, landing 10000 slots below the frame and causing SIGBUS/SIGSEGV.
