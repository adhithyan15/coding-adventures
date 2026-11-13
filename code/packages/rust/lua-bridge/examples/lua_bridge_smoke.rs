// A Lua C module built only to be loaded into a real Lua by tests/smoke.lua.
//
// Why this exists
// ---------------
// lua-bridge is a library of `extern "C"` declarations.  A compiler cannot tell
// whether a declaration names a symbol Lua really exports, whether a constant has
// the value the headers give it, or whether an argument order is right: all of
// those compile.  Six of this crate's declarations (`lua_insert`, `lua_remove`,
// `lua_isboolean`, `luaL_getmetatable`, `luaL_checkstring` and a misspelt
// `lua_newmetatable`) named things that are C *macros* or do not exist, and only
// loading a module that uses them into Lua shows it.  So this module calls every
// part of the bridge that a Lua module needs, and tests/smoke.lua checks the
// results from the Lua side.
//
// Loading it
// ----------
// `package.loadlib(path, "luaopen_lua_bridge_smoke")` finds the exported
// `luaopen_lua_bridge_smoke` below, calls it, and gets back the module table.
//
// The error paths use `lua_error`/`luaL_error`, which unwind with `longjmp`
// across these Rust frames.  Every function that raises holds no value with a
// destructor at that point (only `&'static` data), so skipping its remaining
// frames leaks nothing and breaks no invariant.

#![allow(non_snake_case)]

use lua_bridge::*;
use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The registry name of the Counter userdata's metatable.
const COUNTER_TYPE: *const c_char = c"lua_bridge_smoke.Counter".as_ptr();

/// How many Counter userdata Lua's collector has finalised.
static FINALISED: AtomicUsize = AtomicUsize::new(0);

/// The payload of a Counter userdata: a plain Rust struct living inside a Lua
/// full userdata, which is how a Rust object is exposed to Lua.
struct Counter {
    value: i64,
}

// -- numbers ----------------------------------------------------------------

/// `add(a, b)` -> `a + b`, as floats.  Exercises `luaL_checknumber`.
unsafe extern "C" fn smoke_add(L: *mut lua_State) -> c_int {
    let a = luaL_checknumber(L, 1);
    let b = luaL_checknumber(L, 2);
    lua_pushnumber(L, a + b);
    1
}

/// `add_int(a, b)` -> `a + b`, as integers.  Exercises `luaL_checkinteger`.
unsafe extern "C" fn smoke_add_int(L: *mut lua_State) -> c_int {
    let a = luaL_checkinteger(L, 1);
    let b = luaL_checkinteger(L, 2);
    lua_pushinteger(L, a.wrapping_add(b));
    1
}

/// `to_int(v)` -> `value, ok`.  Exercises `lua_tointegerx` and its `isnum` flag.
unsafe extern "C" fn smoke_to_int(L: *mut lua_State) -> c_int {
    let mut isnum: c_int = 0;
    let value = lua_tointegerx(L, 1, &mut isnum);
    lua_pushinteger(L, value);
    lua_pushboolean(L, isnum);
    2
}

/// `truthy(v)` -> Lua truthiness of `v`.  Exercises `lua_toboolean`.
unsafe extern "C" fn smoke_truthy(L: *mut lua_State) -> c_int {
    let truthy = lua_toboolean(L, 1);
    lua_pushboolean(L, truthy);
    1
}

// -- strings ----------------------------------------------------------------

/// `echo(s)` -> `s`, byte for byte (embedded NULs included), or nil when `s` is
/// not a string.  Exercises `get_str` and `push_str`.
unsafe extern "C" fn smoke_echo(L: *mut lua_State) -> c_int {
    match get_str(L, 1) {
        Some(s) => push_str(L, &s),
        None => lua_pushnil(L),
    }
    1
}

/// `cstring(s)` -> `s` re-pushed as a C string (stops at the first NUL), after
/// `luaL_checkstring` has validated the argument.
unsafe extern "C" fn smoke_cstring(L: *mut lua_State) -> c_int {
    let s = luaL_checkstring(L, 1);
    lua_pushstring(L, s);
    1
}

// -- tables -----------------------------------------------------------------

/// `sum(t)` -> the sum of the numbers in array `t`, or nil when `t` is not a
/// table of numbers.  Exercises `get_f64_table`, `lua_rawlen` and `lua_rawgeti`.
unsafe extern "C" fn smoke_sum(L: *mut lua_State) -> c_int {
    match get_f64_table(L, 1) {
        Some(values) => lua_pushnumber(L, values.iter().sum()),
        None => lua_pushnil(L),
    }
    1
}

/// `range(n)` -> `{1.0, 2.0, ..., n}`.  Exercises `push_f64_table` and
/// `lua_rawseti` (Lua arrays start at 1).
unsafe extern "C" fn smoke_range(L: *mut lua_State) -> c_int {
    let n = luaL_checkinteger(L, 1).max(0);
    let values: Vec<f64> = (1..=n).map(|i| i as f64).collect();
    push_f64_table(L, &values);
    1
}

/// `count_pairs(t)` -> `pairs, len, rawlen`.  Exercises `lua_next`, `lua_len`
/// and `lua_rawlen`.
unsafe extern "C" fn smoke_count_pairs(L: *mut lua_State) -> c_int {
    let mut pairs: lua_Integer = 0;
    lua_pushnil(L); // first key
    while lua_next(L, 1) != 0 {
        pairs += 1;
        lua_pop(L, 1); // drop the value, keep the key for the next lua_next
    }
    lua_len(L, 1);
    let len = lua_tointeger(L, -1);
    lua_pop(L, 1);
    let rawlen = lua_rawlen(L, 1);
    lua_pushinteger(L, pairs);
    lua_pushinteger(L, len);
    lua_pushinteger(L, rawlen);
    3
}

/// `fields()` -> `table, k, answer`.  Builds a table with `lua_setfield` and
/// `lua_settable`, reads it back with `lua_gettable` and `lua_getfield`.
unsafe extern "C" fn smoke_fields(L: *mut lua_State) -> c_int {
    lua_newtable(L); // [t]
    lua_pushinteger(L, 7);
    lua_setfield(L, -2, c"answer".as_ptr()); // t.answer = 7
    push_str(L, "k");
    lua_pushnumber(L, 3.5);
    lua_settable(L, -3); // t["k"] = 3.5
    push_str(L, "k");
    lua_gettable(L, -2); // [t, t.k]
    lua_getfield(L, -2, c"answer".as_ptr()); // [t, t.k, t.answer]
                                             // Return all three values; `t` is at the bottom.
    3
}

/// `registry_globals()` -> the globals table, fetched through the registry.
///
/// Lua keeps `_G` at `registry[LUA_RIDX_GLOBALS]` and `LUA_RIDX_GLOBALS` is 2,
/// so this fails unless `LUA_REGISTRYINDEX` has Lua 5.4's value.
unsafe extern "C" fn smoke_registry_globals(L: *mut lua_State) -> c_int {
    lua_rawgeti(L, LUA_REGISTRYINDEX, 2);
    1
}

// -- the stack --------------------------------------------------------------

/// `stack()` -> `3, 2, 3`.  Exercises `lua_settop`, `lua_insert`, `lua_remove`,
/// `lua_pushvalue` and `lua_gettop`, which are macros or rotations in Lua 5.4.
unsafe extern "C" fn smoke_stack(L: *mut lua_State) -> c_int {
    lua_settop(L, 0);
    lua_pushinteger(L, 1);
    lua_pushinteger(L, 2);
    lua_pushinteger(L, 3); // [1, 2, 3]
    lua_insert(L, 1); // [3, 1, 2]
    lua_remove(L, 2); // [3, 2]
    lua_pushvalue(L, 1); // [3, 2, 3]
    lua_gettop(L)
}

/// `type_name(v)` -> the name of `v`'s type, from `lua_type` and the `lua_is*`
/// predicates.
unsafe extern "C" fn smoke_type_name(L: *mut lua_State) -> c_int {
    let name = match lua_type(L, 1) {
        LUA_TNONE => "none",
        LUA_TNIL => "nil",
        LUA_TBOOLEAN => "boolean",
        LUA_TLIGHTUSERDATA => "lightuserdata",
        LUA_TNUMBER => "number",
        LUA_TSTRING => "string",
        LUA_TTABLE => "table",
        LUA_TFUNCTION => "function",
        LUA_TUSERDATA => "userdata",
        LUA_TTHREAD => "thread",
        _ => "unknown",
    };
    push_str(L, name);
    // The predicates agree with lua_type: report them as a second value, in the
    // order boolean, number, string, userdata, as a 4-character mask.
    let mask: String = [
        lua_isboolean(L, 1),
        lua_isnumber(L, 1),
        lua_isstring(L, 1),
        lua_isuserdata(L, 1),
    ]
    .iter()
    .map(|&b| if b != 0 { '1' } else { '0' })
    .collect();
    push_str(L, &mask);
    2
}

// -- functions --------------------------------------------------------------

unsafe extern "C" fn answer(L: *mut lua_State) -> c_int {
    lua_pushinteger(L, 99);
    1
}

/// `make_function()` -> a Lua function that returns 99.  Exercises
/// `lua_pushcfunction` / `lua_pushcclosure`.
unsafe extern "C" fn smoke_make_function(L: *mut lua_State) -> c_int {
    lua_pushcfunction(L, Some(answer));
    1
}

// -- userdata ---------------------------------------------------------------

/// `Counter(start)` -> a userdata holding a Rust `Counter`, with a metatable
/// that has `__index`, a `bump` method and a `__gc` finaliser.
unsafe extern "C" fn smoke_counter_new(L: *mut lua_State) -> c_int {
    let start = luaL_checkinteger(L, 1);
    let slot = lua_newuserdatauv(L, size_of::<Counter>(), 0) as *mut Counter;
    ptr::write(slot, Counter { value: start });
    luaL_getmetatable(L, COUNTER_TYPE);
    lua_setmetatable(L, -2);
    1
}

/// `counter:bump()` -> the new value.  `luaL_checkudata` raises a Lua error when
/// the receiver is not a Counter.
unsafe extern "C" fn smoke_counter_bump(L: *mut lua_State) -> c_int {
    let counter = luaL_checkudata(L, 1, COUNTER_TYPE) as *mut Counter;
    (*counter).value += 1;
    lua_pushinteger(L, (*counter).value);
    1
}

/// `__gc` for a Counter: runs the Rust destructor, which is what makes a Rust
/// value inside a Lua userdata safe.
unsafe extern "C" fn smoke_counter_gc(L: *mut lua_State) -> c_int {
    let counter = lua_touserdata(L, 1) as *mut Counter;
    if !counter.is_null() {
        ptr::drop_in_place(counter);
        FINALISED.fetch_add(1, Ordering::SeqCst);
    }
    0
}

/// `finalised()` -> how many Counters `__gc` has run for.
unsafe extern "C" fn smoke_finalised(L: *mut lua_State) -> c_int {
    lua_pushinteger(L, FINALISED.load(Ordering::SeqCst) as lua_Integer);
    1
}

/// `has_metatable(v)` -> whether `v` has a metatable (`lua_getmetatable`).
unsafe extern "C" fn smoke_has_metatable(L: *mut lua_State) -> c_int {
    let has = lua_getmetatable(L, 1);
    if has != 0 {
        lua_pop(L, 1); // the metatable lua_getmetatable pushed
    }
    lua_pushboolean(L, has);
    1
}

/// `light()` -> `type, round_trips`: a light userdata pushed with
/// `lua_pushlightuserdata` and read back with `lua_touserdata`.
unsafe extern "C" fn smoke_light(L: *mut lua_State) -> c_int {
    let raw = 0x1234usize as *mut c_void;
    lua_pushlightuserdata(L, raw);
    let ty = lua_type(L, -1);
    let back = lua_touserdata(L, -1);
    lua_pushinteger(L, ty as lua_Integer);
    lua_pushboolean(L, (back == raw) as c_int);
    2
}

// -- errors -----------------------------------------------------------------

/// `raise(msg)` raises `msg` as a Lua error through `raise_error`.
unsafe extern "C" fn smoke_raise(L: *mut lua_State) -> c_int {
    raise_error(L, "boom from rust")
}

/// `raise_formatted()` raises through the variadic `luaL_error`.
unsafe extern "C" fn smoke_raise_formatted(L: *mut lua_State) -> c_int {
    luaL_error(L, c"bad %s %d".as_ptr(), c"thing".as_ptr(), 42 as c_int)
}

/// `raise_argerror()` raises through `luaL_argerror`.
unsafe extern "C" fn smoke_raise_argerror(L: *mut lua_State) -> c_int {
    luaL_argerror(L, 1, c"must be fancier".as_ptr())
}

// -- the module -------------------------------------------------------------

macro_rules! reg {
    ($name:literal, $func:ident) => {
        luaL_Reg {
            name: concat!($name, "\0").as_ptr() as *const c_char,
            func: Some($func),
        }
    };
}

/// The entry point `package.loadlib` looks up: build the module table and the
/// Counter metatable, and return the table.
///
/// Written out by hand: the crate's `lua_module!` macro needs the nightly-only
/// `concat_idents!`, so it cannot be used on the pinned stable toolchain.
///
/// # Safety
///
/// `L` must be the valid `lua_State` Lua passes to a `luaopen_*` function.
#[no_mangle]
pub unsafe extern "C" fn luaopen_lua_bridge_smoke(L: *mut lua_State) -> c_int {
    // The Counter metatable: __index = itself, so `counter:bump()` finds `bump`.
    let methods = [
        reg!("bump", smoke_counter_bump),
        reg!("__gc", smoke_counter_gc),
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];
    luaL_newmetatable(L, COUNTER_TYPE);
    lua_pushvalue(L, -1);
    lua_setfield(L, -2, c"__index".as_ptr());
    luaL_setfuncs(L, methods.as_ptr(), 0);
    lua_pop(L, 1);

    let functions = [
        reg!("add", smoke_add),
        reg!("add_int", smoke_add_int),
        reg!("to_int", smoke_to_int),
        reg!("truthy", smoke_truthy),
        reg!("echo", smoke_echo),
        reg!("cstring", smoke_cstring),
        reg!("sum", smoke_sum),
        reg!("range", smoke_range),
        reg!("count_pairs", smoke_count_pairs),
        reg!("fields", smoke_fields),
        reg!("registry_globals", smoke_registry_globals),
        reg!("stack", smoke_stack),
        reg!("type_name", smoke_type_name),
        reg!("make_function", smoke_make_function),
        reg!("Counter", smoke_counter_new),
        reg!("finalised", smoke_finalised),
        reg!("has_metatable", smoke_has_metatable),
        reg!("light", smoke_light),
        reg!("raise", smoke_raise),
        reg!("raise_formatted", smoke_raise_formatted),
        reg!("raise_argerror", smoke_raise_argerror),
        luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];
    register_lib(L, &functions);
    1
}
