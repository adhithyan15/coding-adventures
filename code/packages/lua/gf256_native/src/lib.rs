//! # gf256_native — Lua C extension wrapping Rust GF(256) arithmetic
//!
//! This Lua module exposes Galois Field GF(2^8) arithmetic to Lua scripts.
//! GF(256) is the finite field with 256 elements, used in Reed-Solomon,
//! QR codes, and AES.
//!
//! ## Element representation
//!
//! GF(256) elements are integers 0–255. They cross the Lua boundary as
//! Lua integers:
//!
//! ```lua
//! gf256_native.add(83, 202)    -- returns 153 (83 XOR 202)
//! ```
//!
//! ## Error handling
//!
//! - `divide(_, 0)` and `inverse(0)` call `raise_error` (Lua `error()`).
//! - Out-of-range integers raise errors via the argument validation helpers
//!   in lua-bridge (`luaL_checkinteger` raises Lua errors natively).

use lua_bridge::{
    lua_Integer, lua_State, lua_pushinteger, luaL_Reg, luaL_checkinteger, raise_error,
    register_lib,
};
use std::ffi::c_int;
use std::panic::catch_unwind;
use std::ptr;

// ---------------------------------------------------------------------------
// Helper: read a GF(256) element from argument n (integer 0–255)
// ---------------------------------------------------------------------------

unsafe fn check_u8(L: *mut lua_State, n: c_int) -> u8 {
    let v = luaL_checkinteger(L, n);
    if v < 0 || v > 255 {
        raise_error(L, "GF256 argument out of range [0, 255]");
    }
    v as u8
}

// ---------------------------------------------------------------------------
// Lua C functions
// ---------------------------------------------------------------------------

/// `gf256_native.add(a, b) → integer`
///
/// Addition in GF(256) = XOR. `add(a, b) = a XOR b`.
unsafe extern "C" fn lua_gf_add(L: *mut lua_State) -> c_int {
    let a = check_u8(L, 1);
    let b = check_u8(L, 2);
    lua_pushinteger(L, gf256::add(a, b) as lua_Integer);
    1
}

/// `gf256_native.subtract(a, b) → integer`
///
/// Subtraction in GF(256) = XOR (same as addition in characteristic 2).
unsafe extern "C" fn lua_gf_subtract(L: *mut lua_State) -> c_int {
    let a = check_u8(L, 1);
    let b = check_u8(L, 2);
    lua_pushinteger(L, gf256::subtract(a, b) as lua_Integer);
    1
}

/// `gf256_native.multiply(a, b) → integer`
///
/// Multiplication using log/antilog tables. O(1) time.
unsafe extern "C" fn lua_gf_multiply(L: *mut lua_State) -> c_int {
    let a = check_u8(L, 1);
    let b = check_u8(L, 2);
    lua_pushinteger(L, gf256::multiply(a, b) as lua_Integer);
    1
}

/// `gf256_native.divide(a, b) → integer`
///
/// Division in GF(256). Raises a Lua error if b is 0.
unsafe extern "C" fn lua_gf_divide(L: *mut lua_State) -> c_int {
    let a = check_u8(L, 1);
    let b = check_u8(L, 2);
    match catch_unwind(|| gf256::divide(a, b)) {
        Ok(result) => {
            lua_pushinteger(L, result as lua_Integer);
            1
        }
        Err(_) => raise_error(L, "GF256: division by zero"),
    }
}

/// `gf256_native.power(base, exp) → integer`
///
/// Raise a GF(256) element to a non-negative integer power.
/// `power(base, 0) = 1` for any base. `power(0, 0) = 1` by convention.
unsafe extern "C" fn lua_gf_power(L: *mut lua_State) -> c_int {
    let base = check_u8(L, 1);
    let exp_raw = luaL_checkinteger(L, 2);
    if exp_raw < 0 {
        raise_error(L, "GF256 power: exponent must be non-negative");
    }
    lua_pushinteger(L, gf256::power(base, exp_raw as u32) as lua_Integer);
    1
}

/// `gf256_native.inverse(a) → integer`
///
/// Multiplicative inverse: `multiply(a, inverse(a)) == 1`.
/// Raises a Lua error if a is 0.
unsafe extern "C" fn lua_gf_inverse(L: *mut lua_State) -> c_int {
    let a = check_u8(L, 1);
    match catch_unwind(|| gf256::inverse(a)) {
        Ok(result) => {
            lua_pushinteger(L, result as lua_Integer);
            1
        }
        Err(_) => raise_error(L, "GF256: zero has no multiplicative inverse"),
    }
}

// ---------------------------------------------------------------------------
// Function registration table
// ---------------------------------------------------------------------------

// luaL_Reg contains *const c_char which is not auto-Sync.
struct FuncTable([luaL_Reg; 7]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    luaL_Reg { name: b"add\0".as_ptr()      as *const _, func: Some(lua_gf_add)      },
    luaL_Reg { name: b"subtract\0".as_ptr() as *const _, func: Some(lua_gf_subtract) },
    luaL_Reg { name: b"multiply\0".as_ptr() as *const _, func: Some(lua_gf_multiply) },
    luaL_Reg { name: b"divide\0".as_ptr()   as *const _, func: Some(lua_gf_divide)   },
    luaL_Reg { name: b"power\0".as_ptr()    as *const _, func: Some(lua_gf_power)    },
    luaL_Reg { name: b"inverse\0".as_ptr()  as *const _, func: Some(lua_gf_inverse)  },
    // Sentinel
    luaL_Reg { name: ptr::null(),            func: None                               },
]);

// ---------------------------------------------------------------------------
// Module entry point
// ---------------------------------------------------------------------------

/// Lua module entry point: called by `require("gf256_native")`.
///
/// # Safety
///
/// Called by the Lua runtime; `L` is guaranteed valid.
#[no_mangle]
pub unsafe extern "C" fn luaopen_gf256_native(L: *mut lua_State) -> c_int {
    register_lib(L, &FUNCS.0);
    1
}
