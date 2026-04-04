//! # polynomial_native — Lua C extension wrapping Rust polynomial arithmetic
//!
//! This file implements a Lua C module (`require("polynomial_native")`) that
//! exposes polynomial arithmetic to Lua scripts.
//!
//! ## The Lua stack model
//!
//! Lua's C API is stack-based. Arguments arrive on the stack:
//!
//! ```text
//! Stack when Lua calls polynomial_native.add(a, b):
//!   [1]  a  ← first argument (the dividend polynomial, a Lua table)
//!   [2]  b  ← second argument
//! ```
//!
//! Each Lua C function:
//! 1. Reads arguments off the stack with `get_f64_table(L, 1)`, etc.
//! 2. Calls the Rust polynomial function.
//! 3. Pushes results with `push_f64_table(L, &result)`.
//! 4. Returns the count of pushed values (usually 1, or 2 for divmod).
//!
//! ## Polynomial representation
//!
//! Polynomials cross the Lua boundary as **Lua tables of numbers** (1-indexed):
//!
//! ```lua
//! local p = {3.0, 0.0, 1.0}  -- represents 3 + 0·x + 1·x² = 3 + x²
//! ```
//!
//! The Rust side uses 0-indexed Vec<f64>; the `get_f64_table`/`push_f64_table`
//! helpers in lua-bridge convert between the two indexing conventions.
//!
//! ## Error handling
//!
//! - Wrong argument type: `raise_error(L, msg)` — calls `lua_error` which
//!   longjmps and never returns (like a Lua `error()` call).
//! - Division by zero polynomial: `std::panic::catch_unwind` catches the Rust
//!   panic, then we call `raise_error` to surface it as a Lua error.
//!
//! ## Module entry point
//!
//! `luaopen_polynomial_native` is the C entry point that Lua calls when the
//! module is first `require()`d. It pushes a table of functions and returns 1.
//! The function name must match the library filename: `polynomial_native.so`
//! → `luaopen_polynomial_native`.

use lua_bridge::{
    get_f64, get_f64_table, lua_Integer, lua_State, lua_pushinteger, lua_pushnumber,
    luaL_Reg, push_f64_table, raise_error, register_lib,
};
use std::ffi::{c_int, c_void};
use std::panic::catch_unwind;
use std::ptr;

// ---------------------------------------------------------------------------
// Helper macro: read a polynomial table from stack argument `n`
// ---------------------------------------------------------------------------

macro_rules! get_poly_arg {
    ($L:expr, $n:expr) => {{
        match unsafe { get_f64_table($L, $n) } {
            Some(v) => v,
            None => unsafe {
                raise_error($L, concat!("argument ", stringify!($n), " must be a table of numbers"))
            },
        }
    }};
}

macro_rules! get_number_arg {
    ($L:expr, $n:expr) => {{
        match unsafe { get_f64($L, $n) } {
            Some(v) => v,
            None => unsafe {
                raise_error($L, concat!("argument ", stringify!($n), " must be a number"))
            },
        }
    }};
}

// ---------------------------------------------------------------------------
// Lua C function implementations
// ---------------------------------------------------------------------------

/// `polynomial_native.normalize(poly) → table`
///
/// Strip trailing near-zero coefficients from a polynomial.
unsafe extern "C" fn lua_normalize(L: *mut lua_State) -> c_int {
    let poly = get_poly_arg!(L, 1);
    let result = polynomial::normalize(&poly);
    push_f64_table(L, &result);
    1
}

/// `polynomial_native.degree(poly) → integer`
///
/// Return the degree (highest non-zero exponent index).
unsafe extern "C" fn lua_degree(L: *mut lua_State) -> c_int {
    let poly = get_poly_arg!(L, 1);
    let d = polynomial::degree(&poly) as lua_Integer;
    lua_pushinteger(L, d);
    1
}

/// `polynomial_native.zero() → table`
///
/// Return the zero polynomial `{0.0}`.
unsafe extern "C" fn lua_zero(L: *mut lua_State) -> c_int {
    push_f64_table(L, &polynomial::zero());
    1
}

/// `polynomial_native.one() → table`
///
/// Return the one polynomial `{1.0}`.
unsafe extern "C" fn lua_one(L: *mut lua_State) -> c_int {
    push_f64_table(L, &polynomial::one());
    1
}

/// `polynomial_native.add(a, b) → table`
unsafe extern "C" fn lua_add(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    push_f64_table(L, &polynomial::add(&a, &b));
    1
}

/// `polynomial_native.subtract(a, b) → table`
unsafe extern "C" fn lua_subtract(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    push_f64_table(L, &polynomial::subtract(&a, &b));
    1
}

/// `polynomial_native.multiply(a, b) → table`
unsafe extern "C" fn lua_multiply(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    push_f64_table(L, &polynomial::multiply(&a, &b));
    1
}

/// `polynomial_native.divmod(a, b) → table, table`
///
/// Returns TWO values on the stack: quotient and remainder.
/// `local q, r = polynomial_native.divmod(a, b)`
unsafe extern "C" fn lua_divmod(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    match catch_unwind(|| polynomial::divmod(&a, &b)) {
        Ok((q, r)) => {
            push_f64_table(L, &q);
            push_f64_table(L, &r);
            2 // Two return values
        }
        Err(_) => raise_error(L, "polynomial division by zero"),
    }
}

/// `polynomial_native.divide(a, b) → table`
unsafe extern "C" fn lua_divide(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    match catch_unwind(|| polynomial::divide(&a, &b)) {
        Ok(q) => {
            push_f64_table(L, &q);
            1
        }
        Err(_) => raise_error(L, "polynomial division by zero"),
    }
}

/// `polynomial_native.modulo(a, b) → table`
unsafe extern "C" fn lua_modulo(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    match catch_unwind(|| polynomial::modulo(&a, &b)) {
        Ok(r) => {
            push_f64_table(L, &r);
            1
        }
        Err(_) => raise_error(L, "polynomial division by zero"),
    }
}

/// `polynomial_native.evaluate(poly, x) → number`
unsafe extern "C" fn lua_evaluate(L: *mut lua_State) -> c_int {
    let poly = get_poly_arg!(L, 1);
    let x = get_number_arg!(L, 2);
    lua_pushnumber(L, polynomial::evaluate(&poly, x));
    1
}

/// `polynomial_native.gcd(a, b) → table`
unsafe extern "C" fn lua_gcd(L: *mut lua_State) -> c_int {
    let a = get_poly_arg!(L, 1);
    let b = get_poly_arg!(L, 2);
    push_f64_table(L, &polynomial::gcd(&a, &b));
    1
}

// ---------------------------------------------------------------------------
// Function registration table
// ---------------------------------------------------------------------------
//
// A null-terminated array of luaL_Reg entries. `register_lib` in lua-bridge
// creates a Lua table and registers these functions into it.
//
// The `name` fields are null-terminated byte strings, cast to *const c_char.

// luaL_Reg contains *const c_char which is not auto-Sync.
// Wrap the array in a newtype and assert Sync manually.
// Safety: immutable static data; read once during Lua module load.
struct FuncTable([luaL_Reg; 13]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    luaL_Reg { name: b"normalize\0".as_ptr() as *const _, func: Some(lua_normalize) },
    luaL_Reg { name: b"degree\0".as_ptr()    as *const _, func: Some(lua_degree)    },
    luaL_Reg { name: b"zero\0".as_ptr()      as *const _, func: Some(lua_zero)      },
    luaL_Reg { name: b"one\0".as_ptr()       as *const _, func: Some(lua_one)       },
    luaL_Reg { name: b"add\0".as_ptr()       as *const _, func: Some(lua_add)       },
    luaL_Reg { name: b"subtract\0".as_ptr()  as *const _, func: Some(lua_subtract)  },
    luaL_Reg { name: b"multiply\0".as_ptr()  as *const _, func: Some(lua_multiply)  },
    luaL_Reg { name: b"divmod\0".as_ptr()    as *const _, func: Some(lua_divmod)    },
    luaL_Reg { name: b"divide\0".as_ptr()    as *const _, func: Some(lua_divide)    },
    luaL_Reg { name: b"modulo\0".as_ptr()    as *const _, func: Some(lua_modulo)    },
    luaL_Reg { name: b"evaluate\0".as_ptr()  as *const _, func: Some(lua_evaluate)  },
    luaL_Reg { name: b"gcd\0".as_ptr()       as *const _, func: Some(lua_gcd)       },
    // Sentinel: null name signals end of array to luaL_setfuncs.
    luaL_Reg { name: ptr::null(),             func: None                             },
]);

// ---------------------------------------------------------------------------
// Module entry point
// ---------------------------------------------------------------------------
//
// Lua calls this function when `require("polynomial_native")` is executed.
// The name MUST be `luaopen_<library_name>` where the library name matches
// the `.so` filename (without the `lib` prefix and `.so` extension).
//
// We write this by hand rather than using the `lua_module!` macro because
// the macro depends on `concat_idents`, a proc-macro crate not yet in
// stable Rust.

/// Lua module entry point: called by `require("polynomial_native")`.
///
/// Registers all polynomial functions into a Lua table and pushes it onto
/// the stack. Returns 1 (one value on the stack = the module table).
///
/// # Safety
///
/// Called by the Lua runtime; `L` is guaranteed valid.
#[no_mangle]
pub unsafe extern "C" fn luaopen_polynomial_native(L: *mut lua_State) -> c_int {
    register_lib(L, &FUNCS.0);
    1
}
