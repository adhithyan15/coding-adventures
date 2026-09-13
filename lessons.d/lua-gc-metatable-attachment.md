---
category: Native extensions & FFI
---

# Lua `__gc` metatable attachment

: do NOT `push_cstr("__gc")` before `lua_rawset_str_top` — the function supplies the key. Pattern: `luaL_newmetatable; lua_pushcclosure(gc_fn); lua_rawset_str_top(-2, "__gc\0"); lua_setmetatable(-2)`.
