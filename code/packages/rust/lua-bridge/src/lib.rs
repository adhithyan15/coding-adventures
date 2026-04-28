// Lua's C API conventionally uses uppercase `L` for the lua_State pointer,
// and non-snake-case names like `luaL_Reg`. Allow these to match C API docs.
#![allow(non_snake_case, non_camel_case_types)]

//! # lua-bridge — Zero-dependency Rust wrapper for Lua 5.4 C API
//!
//! This crate provides safe Rust wrappers around Lua's C extension API
//! using raw `extern "C"` declarations. No mlua, no rlua, no bindgen,
//! no build-time header requirements. Compiles on any platform with just
//! a Rust toolchain.
//!
//! ## How it works
//!
//! Lua C modules are shared libraries that export a `luaopen_<name>` function.
//! When `require("mymod")` is called, Lua's dynamic loader loads the `.so`/`.dll`,
//! finds `luaopen_mymod`, and calls it. Our extern "C" declarations are resolved
//! against the running `liblua` at load time — no headers needed.
//!
//! ## The Lua stack model
//!
//! Lua is a **stack-based** language API. Every value is accessed via stack
//! indices, not pointers. The stack grows upward; index 1 is the bottom,
//! index -1 is the top. This is the fundamental mental model:
//!
//! ```text
//! Stack (top = -1 = idx 3):
//!   [1]  42          -- integer
//!   [2]  "hello"     -- string
//!   [3]  {1,2,3}     -- table   <-- top (-1)
//! ```
//!
//! - Push: adds to the top; count goes up.
//! - Pop: removes from the top; count goes down.
//! - `lua_gettop` returns the current count (= index of top element).
//!
//! ## C functions vs. macros
//!
//! Lua's C API mixes real functions with C preprocessor macros. Macros like
//! `lua_pop`, `lua_newtable`, and `lua_pushcfunction` don't exist as exported
//! symbols. We implement them as inline Rust functions that call the underlying
//! real functions.
//!
//! ## Why zero dependencies?
//!
//! - **Compiles everywhere** — no Lua headers needed at build time
//! - **No mlua macros** — every symbol is explicit and grep-able
//! - **No version conflicts** — targets the stable Lua 5.4 ABI
//! - **Fully auditable** — no proc-macro magic

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Core Lua types
// ---------------------------------------------------------------------------

/// Opaque Lua interpreter state.
///
/// Every Lua function takes a `*mut lua_State` as its first argument.
/// This is the Lua equivalent of Python's `PyObject*` — it carries the
/// entire interpreter: stack, global table, GC state, etc.
#[repr(C)]
pub struct lua_State {
    _opaque: [u8; 0],
}

/// A Lua C function — the signature every Lua-callable Rust function must have.
///
/// Returns the number of values it pushed onto the stack (its return value count).
/// Lua uses multiple return values natively; this is how you return them.
pub type lua_CFunction = Option<unsafe extern "C" fn(L: *mut lua_State) -> c_int>;

/// Lua's floating-point type (`double` by default).
pub type lua_Number = f64;

/// Lua's integer type (`long long` in Lua 5.3+).
pub type lua_Integer = i64;

/// A `luaL_Reg` entry describes one function in a library registration table.
///
/// Build a null-terminated array of these and pass to `luaL_setfuncs` to
/// register an entire module at once. The last entry must have `name = null`.
#[repr(C)]
pub struct luaL_Reg {
    /// Function name as a null-terminated C string. Null signals end of array.
    pub name: *const c_char,
    /// The Rust function implementing this Lua function.
    pub func: lua_CFunction,
}

// ---------------------------------------------------------------------------
// Stack pseudo-indices
// ---------------------------------------------------------------------------
//
// These "negative" indices below LUA_REGISTRYINDEX refer to special tables,
// not actual stack slots. They let you access the registry and upvalues using
// the same index-based API as the regular stack.

/// Pseudo-index for the Lua registry — a global table for C libraries to store values.
///
/// Lua 5.4 derives this from `LUAI_MAXSTACK` (1_000_000): `LUA_REGISTRYINDEX = -(LUAI_MAXSTACK + 1000) = -1_001_000`.
/// The Lua 5.1 value (-10000) is wrong for 5.4 and causes crashes in `luaL_ref`/`luaL_unref`.
pub const LUA_REGISTRYINDEX: c_int = -1_001_000;
/// Pseudo-index for the C function's environment table (Lua 5.1 only; removed in 5.2+).
#[deprecated = "LUA_ENVIRONINDEX was removed in Lua 5.2; use upvalues in Lua 5.4"]
pub const LUA_ENVIRONINDEX: c_int = -1_001_001;
/// Pseudo-index for the global table `_G` (Lua 5.1 only; removed in 5.2+).
#[deprecated = "LUA_GLOBALSINDEX was removed in Lua 5.2; use lua_getglobal in Lua 5.4"]
pub const LUA_GLOBALSINDEX: c_int = -1_001_002;

// ---------------------------------------------------------------------------
// Lua value type tags (returned by lua_type)
// ---------------------------------------------------------------------------
//
// `lua_type(L, idx)` returns one of these constants describing what kind of
// value sits at stack index `idx`.

/// No value at this index (index out of range).
pub const LUA_TNONE: c_int = -1;
/// The Lua `nil` type.
pub const LUA_TNIL: c_int = 0;
/// The Lua boolean type.
pub const LUA_TBOOLEAN: c_int = 1;
/// A light userdata (raw pointer, not GC-managed).
pub const LUA_TLIGHTUSERDATA: c_int = 2;
/// A Lua number (integer or float).
pub const LUA_TNUMBER: c_int = 3;
/// A Lua string.
pub const LUA_TSTRING: c_int = 4;
/// A Lua table.
pub const LUA_TTABLE: c_int = 5;
/// A Lua function (Lua-defined or C).
pub const LUA_TFUNCTION: c_int = 6;
/// A full userdata (GC-managed block of memory, like a resource).
pub const LUA_TUSERDATA: c_int = 7;
/// A Lua coroutine (thread).
pub const LUA_TTHREAD: c_int = 8;

// ---------------------------------------------------------------------------
// Return value / status constants
// ---------------------------------------------------------------------------

/// Pass as `nresults` to `lua_call` to accept any number of return values.
pub const LUA_MULTRET: c_int = -1;
/// Success status code from `lua_pcall`, `lua_load`, etc.
pub const LUA_OK: c_int = 0;

// ---------------------------------------------------------------------------
// Lua C API — extern "C" declarations (real functions, not macros)
// ---------------------------------------------------------------------------

extern "C" {
    // -- Stack manipulation ------------------------------------------------

    /// Return the index of the top element (= number of values on the stack).
    /// Index 0 means the stack is empty.
    pub fn lua_gettop(L: *mut lua_State) -> c_int;

    /// Set the top of the stack to `idx`.
    /// - Positive `idx`: set stack to exactly that many elements (pads with nil).
    /// - Zero: empty the stack.
    /// - Negative `idx`: relative to current top (use for popping).
    pub fn lua_settop(L: *mut lua_State, idx: c_int);

    /// Push a copy of the value at `idx` onto the stack.
    pub fn lua_pushvalue(L: *mut lua_State, idx: c_int);

    /// Remove the element at `idx`, shifting elements above it down.
    pub fn lua_remove(L: *mut lua_State, idx: c_int);

    /// Move the top element to position `idx`, shifting elements above up.
    pub fn lua_insert(L: *mut lua_State, idx: c_int);

    // -- Push values -------------------------------------------------------

    /// Push `nil` onto the stack.
    pub fn lua_pushnil(L: *mut lua_State);

    /// Push a boolean onto the stack. In Lua, any non-nil, non-false is truthy.
    pub fn lua_pushboolean(L: *mut lua_State, b: c_int);

    /// Push a Lua integer (i64) onto the stack.
    pub fn lua_pushinteger(L: *mut lua_State, n: lua_Integer);

    /// Push a Lua number (f64) onto the stack.
    pub fn lua_pushnumber(L: *mut lua_State, n: lua_Number);

    /// Push a null-terminated C string as a Lua string. Returns a pointer to
    /// the internalized copy (owned by Lua, not the original `s`).
    pub fn lua_pushstring(L: *mut lua_State, s: *const c_char) -> *const c_char;

    /// Push a string with explicit byte length (may contain null bytes).
    pub fn lua_pushlstring(L: *mut lua_State, s: *const c_char, len: usize) -> *const c_char;

    /// Push a C closure with `n` upvalues. The upvalues are popped from the stack.
    /// For a plain C function with no upvalues, use `lua_pushcfunction` (below).
    pub fn lua_pushcclosure(L: *mut lua_State, f: lua_CFunction, n: c_int);

    /// Push a raw pointer as a light userdata. Not GC-managed.
    pub fn lua_pushlightuserdata(L: *mut lua_State, p: *mut c_void);

    // -- Read values -------------------------------------------------------

    /// Return the type tag of the value at `idx`.
    pub fn lua_type(L: *mut lua_State, idx: c_int) -> c_int;

    /// Return 1 if the value at `idx` is a number (integer or float).
    pub fn lua_isnumber(L: *mut lua_State, idx: c_int) -> c_int;

    /// Return 1 if the value at `idx` is a string or a number (coercible).
    pub fn lua_isstring(L: *mut lua_State, idx: c_int) -> c_int;

    /// Return 1 if the value at `idx` is a boolean.
    pub fn lua_isboolean(L: *mut lua_State, idx: c_int) -> c_int;

    /// Return 1 if the value at `idx` is a userdata (full or light).
    pub fn lua_isuserdata(L: *mut lua_State, idx: c_int) -> c_int;

    /// Convert the value at `idx` to a boolean. In Lua, nil and false are
    /// falsy; everything else (including 0) is truthy.
    pub fn lua_toboolean(L: *mut lua_State, idx: c_int) -> c_int;

    /// Convert the value at `idx` to an integer. If `isnum` is non-null,
    /// it is set to 1 on success. Use `lua_tointeger` (below) for convenience.
    pub fn lua_tointegerx(
        L: *mut lua_State,
        idx: c_int,
        isnum: *mut c_int,
    ) -> lua_Integer;

    /// Convert the value at `idx` to a number. If `isnum` is non-null,
    /// it is set to 1 on success. Use `lua_tonumber` (below) for convenience.
    pub fn lua_tonumberx(
        L: *mut lua_State,
        idx: c_int,
        isnum: *mut c_int,
    ) -> lua_Number;

    /// Convert the value at `idx` to a string. If `len` is non-null,
    /// it receives the string length. Returns null if the value is not a string.
    /// The returned pointer is owned by Lua; do not free it.
    pub fn lua_tolstring(
        L: *mut lua_State,
        idx: c_int,
        len: *mut usize,
    ) -> *const c_char;

    /// Return the raw userdata pointer at `idx`, or null if not userdata.
    pub fn lua_touserdata(L: *mut lua_State, idx: c_int) -> *mut c_void;

    // -- Tables ------------------------------------------------------------
    //
    // Lua tables are the universal data structure — arrays, maps, objects,
    // modules — everything is a table. They map any key to any value.

    /// Create a new table, preallocating `narr` array slots and `nrec` hash slots.
    /// Use `lua_newtable` (below) for a plain empty table.
    pub fn lua_createtable(L: *mut lua_State, narr: c_int, nrec: c_int);

    /// Set `table[key] = value` where key is at stack[-2] and value at stack[-1].
    /// Pops both key and value. Table is at `idx`.
    pub fn lua_settable(L: *mut lua_State, idx: c_int);

    /// Push `table[key]` where key is at the top of the stack. Pops the key.
    pub fn lua_gettable(L: *mut lua_State, idx: c_int) -> c_int;

    /// Set `table[k] = value` where value is at the top of the stack. Pops value.
    pub fn lua_setfield(L: *mut lua_State, idx: c_int, k: *const c_char);

    /// Push `table[k]` onto the stack.
    pub fn lua_getfield(L: *mut lua_State, idx: c_int, k: *const c_char) -> c_int;

    /// Set `table[n] = value` (integer key). Pops value. Raw (no metamethods).
    pub fn lua_rawseti(L: *mut lua_State, idx: c_int, n: lua_Integer);

    /// Push `table[n]` (integer key). Raw (no metamethods).
    pub fn lua_rawgeti(L: *mut lua_State, idx: c_int, n: lua_Integer) -> c_int;

    /// Table iteration: push next key-value pair after the key at stack top.
    /// Returns 0 when there are no more pairs.
    pub fn lua_next(L: *mut lua_State, idx: c_int) -> c_int;

    /// Push the length of the value at `idx` (like Lua's `#` operator).
    pub fn lua_len(L: *mut lua_State, idx: c_int);

    /// Return the raw length of a table or string at `idx` (no `__len` metamethod).
    pub fn lua_rawlen(L: *mut lua_State, idx: c_int) -> lua_Integer;

    // -- Userdata / metatables ---------------------------------------------
    //
    // Full userdata is a GC-managed block of memory. You store your Rust
    // struct pointer there and attach a metatable with a `__gc` method to
    // handle cleanup when the GC collects it.

    /// Allocate a new full userdata of `size` bytes with `nuvalue` user values.
    /// Returns a pointer to the raw memory block (fill it with your data).
    pub fn lua_newuserdatauv(L: *mut lua_State, size: usize, nuvalue: c_int) -> *mut c_void;

    /// Create a new empty metatable named `tname` in the registry (returns 1
    /// if new, 0 if it already existed). Pushes the table.
    pub fn lua_newmetatable(L: *mut lua_State, tname: *const c_char) -> c_int;

    /// Pop a table from the stack and set it as the metatable for the value at `idx`.
    pub fn lua_setmetatable(L: *mut lua_State, idx: c_int) -> c_int;

    /// Push the metatable of the value at `idx`. Returns 1 if it has one, 0 if not.
    pub fn lua_getmetatable(L: *mut lua_State, idx: c_int) -> c_int;

    /// Push the metatable registered as `tname` from the registry.
    pub fn luaL_getmetatable(L: *mut lua_State, tname: *const c_char) -> c_int;

    /// Check that the value at `ud` is a userdata of type `tname`. Raises a
    /// Lua error if not. Returns the raw userdata pointer.
    pub fn luaL_checkudata(
        L: *mut lua_State,
        ud: c_int,
        tname: *const c_char,
    ) -> *mut c_void;

    // -- Error / exception -------------------------------------------------
    //
    // Lua uses `longjmp` for error handling, similar to how Ruby uses it.
    // These functions never return in the C sense; they unwind the Lua call stack.

    /// Raise the value at the top of the stack as a Lua error. Never returns.
    pub fn lua_error(L: *mut lua_State) -> c_int;

    /// Raise a formatted Lua error. Never returns.
    pub fn luaL_error(L: *mut lua_State, fmt: *const c_char, ...) -> c_int;

    /// Raise a Lua argument error for argument `narg`. Never returns.
    pub fn luaL_argerror(
        L: *mut lua_State,
        narg: c_int,
        extramsg: *const c_char,
    ) -> c_int;

    /// Check that argument `narg` is an integer; raise an error if not.
    pub fn luaL_checkinteger(L: *mut lua_State, narg: c_int) -> lua_Integer;

    /// Check that argument `narg` is a number; raise an error if not.
    pub fn luaL_checknumber(L: *mut lua_State, narg: c_int) -> lua_Number;

    /// Check that argument `narg` is a string; raise an error if not.
    pub fn luaL_checkstring(L: *mut lua_State, narg: c_int) -> *const c_char;

    // -- Library registration ----------------------------------------------

    /// Register functions from a null-terminated `luaL_Reg` array with `nup`
    /// upvalues into the table on the top of the stack. Pops the upvalues.
    pub fn luaL_setfuncs(L: *mut lua_State, l: *const luaL_Reg, nup: c_int);
}

// ---------------------------------------------------------------------------
// Lua macro equivalents — implemented as inline Rust functions
// ---------------------------------------------------------------------------
//
// These are C macros in the Lua headers, so they don't appear as exported
// symbols. We replicate their logic here as zero-cost inline functions.

/// Pop `n` values from the stack.
///
/// C macro: `#define lua_pop(L,n) lua_settop(L, -(n)-1)`
#[inline]
pub unsafe fn lua_pop(L: *mut lua_State, n: c_int) {
    lua_settop(L, -(n) - 1);
}

/// Create a new empty table and push it.
///
/// C macro: `#define lua_newtable(L) lua_createtable(L, 0, 0)`
#[inline]
pub unsafe fn lua_newtable(L: *mut lua_State) {
    lua_createtable(L, 0, 0);
}

/// Push a C function with no upvalues (the common case).
///
/// C macro: `#define lua_pushcfunction(L,f) lua_pushcclosure(L, (f), 0)`
#[inline]
pub unsafe fn lua_pushcfunction(L: *mut lua_State, f: lua_CFunction) {
    lua_pushcclosure(L, f, 0);
}

/// Convert the value at `idx` to an integer (convenience, ignores isnum).
///
/// C macro: `#define lua_tointeger(L,i) lua_tointegerx(L,(i),NULL)`
#[inline]
pub unsafe fn lua_tointeger(L: *mut lua_State, idx: c_int) -> lua_Integer {
    lua_tointegerx(L, idx, ptr::null_mut())
}

/// Convert the value at `idx` to a number (convenience, ignores isnum).
///
/// C macro: `#define lua_tonumber(L,i) lua_tonumberx(L,(i),NULL)`
#[inline]
pub unsafe fn lua_tonumber(L: *mut lua_State, idx: c_int) -> lua_Number {
    lua_tonumberx(L, idx, ptr::null_mut())
}

/// Convert the value at `idx` to a string pointer (convenience, ignores len).
///
/// C macro: `#define lua_tostring(L,i) lua_tolstring(L,(i),NULL)`
#[inline]
pub unsafe fn lua_tostring(L: *mut lua_State, idx: c_int) -> *const c_char {
    lua_tolstring(L, idx, ptr::null_mut())
}

// ---------------------------------------------------------------------------
// Safe helper functions — the "bridge" layer
// ---------------------------------------------------------------------------

/// Push a Rust `&str` as a Lua string.
///
/// Handles the `CString` conversion and calls `lua_pushlstring` with the
/// correct byte length (no null-terminator required for `pushlstring`).
pub unsafe fn push_str(L: *mut lua_State, s: &str) {
    lua_pushlstring(L, s.as_ptr() as *const c_char, s.len());
}

/// Get the Lua string at stack index `idx` as a Rust `String`.
///
/// Returns `None` if the value is not a string (and not a number, which Lua
/// can coerce). Makes a heap copy — the Lua string is freed when the GC runs.
pub unsafe fn get_str(L: *mut lua_State, idx: c_int) -> Option<String> {
    let mut len: usize = 0;
    let ptr = lua_tolstring(L, idx, &mut len);
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    String::from_utf8(bytes.to_vec()).ok()
}

/// Get the Lua number at stack index `idx` as an `f64`.
///
/// Returns `None` if the value is not a number.
pub unsafe fn get_f64(L: *mut lua_State, idx: c_int) -> Option<f64> {
    if lua_isnumber(L, idx) == 0 {
        return None;
    }
    Some(lua_tonumber(L, idx))
}

/// Push a Rust `&[f64]` as a Lua table of numbers (1-indexed, Lua convention).
///
/// In Lua, arrays are tables with integer keys starting at 1. This function
/// creates a table with `values.len()` sequential entries.
pub unsafe fn push_f64_table(L: *mut lua_State, values: &[f64]) {
    lua_createtable(L, values.len() as c_int, 0);
    for (i, &v) in values.iter().enumerate() {
        lua_pushnumber(L, v);
        lua_rawseti(L, -2, (i + 1) as lua_Integer); // Lua 1-indexed
    }
}

/// Read a Lua table of numbers at stack index `idx` into a `Vec<f64>`.
///
/// Returns `None` if the value is not a table or if any element is not a number.
/// Reads entries at integer keys 1, 2, ..., `#table` (Lua array convention).
pub unsafe fn get_f64_table(L: *mut lua_State, idx: c_int) -> Option<Vec<f64>> {
    if lua_type(L, idx) != LUA_TTABLE {
        return None;
    }
    let len = lua_rawlen(L, idx);
    let mut result = Vec::with_capacity(len as usize);
    for i in 1..=len {
        lua_rawgeti(L, idx, i);
        let v = get_f64(L, -1)?;
        result.push(v);
        lua_pop(L, 1);
    }
    Some(result)
}

/// Raise a Lua error with a Rust string message. Never returns.
///
/// This pushes `msg` as a Lua string and calls `lua_error`, which `longjmp`s
/// out of the current Lua call frame.
pub unsafe fn raise_error(L: *mut lua_State, msg: &str) -> ! {
    push_str(L, msg);
    lua_error(L);
    // lua_error longjmps and never returns; this line is unreachable.
    // The `-> !` return type tells Rust's type checker.
    unreachable!("lua_error never returns")
}

/// Register a module: create a table with the given functions and leave it on the stack.
///
/// `fns` must be a slice whose last entry has `name = null` (the sentinel).
/// This matches the `luaL_newlib` C macro pattern.
///
/// After this call, the function table is on top of the stack, ready to
/// return from `luaopen_<name>`.
pub unsafe fn register_lib(L: *mut lua_State, fns: &[luaL_Reg]) {
    lua_createtable(L, 0, fns.len() as c_int - 1); // -1 for sentinel
    luaL_setfuncs(L, fns.as_ptr(), 0);
}

// ---------------------------------------------------------------------------
// luaopen! — the module entry point macro
// ---------------------------------------------------------------------------
//
// Every Lua C module must export a `luaopen_<name>` function.
// When `require("mymod")` is called, Lua looks for and calls this function.
// It should push one value (the module table) and return 1.
//
// Usage:
//
// ```rust,ignore
// lua_module!(mymod, |L| {
//     register_lib(L, &MY_FUNCS);
//     1  // return 1 value (the table)
// });
// ```

/// Define the Lua module entry point `luaopen_<name>`.
///
/// `$name` must match the file name (without extension) that Lua uses to
/// look up the module. The body closure receives `L: *mut lua_State` and
/// must return a `c_int` (number of values pushed as return values).
///
/// # Example
///
/// ```rust,ignore
/// static FUNCS: &[luaL_Reg] = &[
///     luaL_Reg { name: b"add\0".as_ptr() as *const _, func: Some(lua_add) },
///     luaL_Reg { name: ptr::null(), func: None }, // sentinel
/// ];
///
/// lua_module!(mymath, |L| {
///     register_lib(L, FUNCS);
///     1
/// });
/// ```
#[macro_export]
macro_rules! lua_module {
    ($name:ident, |$L:ident| $body:block) => {
        ::std::concat_idents::concat_idents!(fn_name = luaopen_, $name {
            #[no_mangle]
            pub unsafe extern "C" fn fn_name(
                $L: *mut $crate::lua_State,
            ) -> ::std::ffi::c_int {
                $body
            }
        });
    };
}
