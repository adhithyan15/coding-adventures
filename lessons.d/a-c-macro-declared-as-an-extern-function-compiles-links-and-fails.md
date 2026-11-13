---
category: Native extensions & FFI
---

# A C macro declared as an extern function compiles, links and fails only when the host loads the module, so list a bridge's declarations against the host's exported symbols

The last three echo-only bridge crates (`lua-bridge`, `erl-nif-bridge`,
`perl-bridge`) were given real recipes with a smoke module loaded into the real
interpreter. Two of the three had declared C *macros* as exported functions:

- `lua-bridge`: `lua_insert`, `lua_remove`, `lua_isboolean`, `luaL_getmetatable` and
  `luaL_checkstring` are macros in Lua 5.4, and `lua_newmetatable` does not exist
  (the function is `luaL_newmetatable`). A module that used any of them failed at
  `package.loadlib` with `undefined symbol: lua_insert`. `lua/conduit` had already hit
  one of them and silently worked around it with its own declaration.
- `erl-nif-bridge`: `enif_make_pid` is a macro in OTP's `erl_nif_api_funcs.h`
  (`#define enif_make_pid(ENV, PID) ((const ERL_NIF_TERM)((PID)->pid))`), not a symbol of
  `beam.smp`, so `erlang:load_nif/2` failed with `undefined symbol: enif_make_pid`.
  The Windows half (`win_dyn`) had the right definition; the Unix `extern` block did not.

Nothing in `cargo build`, clippy or a unit test can see this: an `extern "C"` function
that is never called leaves no undefined symbol in the library, and one that is called
only fails when the host `dlopen`s the module with `RTLD_NOW`. Two checks find it:

1. Load a smoke module that calls every declared function into the real host (the
   recipes now do).
2. When writing or reviewing a bridge, compare its declarations with what the host
   exports: `nm -D liblua5.4.so.0 | awk '$2=="T"{print $3}'` (strip the `@@LUA_5.4`
   version suffix) or `nm -D .../beam.smp` against the names in the `extern` block;
   `comm -23 declared exported` lists the ones that will not load. The header says which
   names are macros (`#define lua_insert(L,idx) lua_rotate(L,(idx),1)`), and a macro
   belongs in the crate as an inline Rust function with the macro's body.

Also check the interpreter version first: this sandbox's `lua` was Lua 5.1, which fails
the 5.4 smoke test with `undefined symbol: lua_rotate`, so `tests/smoke.lua` now checks
`_VERSION` and says so.
