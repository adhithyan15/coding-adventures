---
category: Native extensions & FFI
---

# Lua userdata GC + raw `luaL_ref` integers

: integer slots aren't tracked by the GC. If Rust holds `i32` registry refs derived from a userdata's state, pin the userdata itself in the registry (extra `lua_pushvalue` + `luaL_ref`) and unref it only after all integer refs retire — otherwise Linux's aggressive incremental GC collects the parent and your slots become nil mid-flight.
