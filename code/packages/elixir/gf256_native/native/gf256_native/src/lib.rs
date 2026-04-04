//! # gf256_native — Erlang NIF wrapping Rust GF(256) arithmetic
//!
//! This NIF bridges the Rust `gf256` crate into Elixir. GF(256) is the
//! Galois Field with 256 elements, used in Reed-Solomon error correction,
//! QR codes, and AES encryption.
//!
//! ## Element representation
//!
//! GF(256) elements are bytes: integers in the range 0–255. They cross the
//! NIF boundary as **Erlang integers**:
//!
//! ```text
//! Elixir: 83 (integer)  →  Rust: 83u8  →  GF(256) element
//! ```
//!
//! ## Error handling
//!
//! - `divide(_, 0)` and `inverse(0)` panic in Rust (undefined operations).
//!   We wrap them in `catch_unwind` and return `badarg` to Elixir.
//! - Out-of-range integers (< 0 or > 255) → `badarg`.
//!
//! ## The underlying math
//!
//! GF(256) arithmetic works modulo the primitive polynomial
//! `p(x) = x⁸ + x⁴ + x³ + x² + 1` (value 0x11D). Addition is XOR;
//! multiplication uses precomputed log/antilog tables.

#![allow(non_snake_case)]

use erl_nif_bridge::{
    badarg, get_i64, make_i64, ErlNifEnv, ErlNifFunc, ERL_NIF_TERM,
};
use std::ffi::c_int;
use std::panic::catch_unwind;

// ---------------------------------------------------------------------------
// Helper: extract a GF(256) element (0–255) from argv[idx]
// ---------------------------------------------------------------------------

macro_rules! get_u8 {
    ($env:expr, $argv:expr, $idx:expr) => {{
        let term = unsafe { *$argv.add($idx) };
        match unsafe { get_i64($env, term) } {
            Some(v) if v >= 0 && v <= 255 => v as u8,
            _ => return unsafe { badarg($env) },
        }
    }};
}

// ---------------------------------------------------------------------------
// NIF implementations
// ---------------------------------------------------------------------------

/// `add(a, b) → integer`
///
/// Add two GF(256) elements. In characteristic 2, addition is XOR:
/// `add(a, b) = a XOR b`. No tables needed.
pub unsafe extern "C" fn nif_add(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_u8!(env, argv, 0);
    let b = get_u8!(env, argv, 1);
    make_i64(env, gf256::add(a, b) as i64)
}

/// `subtract(a, b) → integer`
///
/// Subtract in GF(256). In characteristic 2, subtraction equals addition
/// (XOR), because `-x = x` for all x.
pub unsafe extern "C" fn nif_subtract(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_u8!(env, argv, 0);
    let b = get_u8!(env, argv, 1);
    make_i64(env, gf256::subtract(a, b) as i64)
}

/// `multiply(a, b) → integer`
///
/// Multiply two GF(256) elements using log/antilog tables. O(1) time.
/// `multiply(a, 0) = 0` for all a.
pub unsafe extern "C" fn nif_multiply(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_u8!(env, argv, 0);
    let b = get_u8!(env, argv, 1);
    make_i64(env, gf256::multiply(a, b) as i64)
}

/// `divide(a, b) → integer`
///
/// Divide a by b in GF(256). Returns `badarg` if b is 0 (undefined).
/// `divide(0, b) = 0` for any non-zero b.
pub unsafe extern "C" fn nif_divide(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_u8!(env, argv, 0);
    let b = get_u8!(env, argv, 1);
    match catch_unwind(|| gf256::divide(a, b)) {
        Ok(result) => make_i64(env, result as i64),
        Err(_) => badarg(env),
    }
}

/// `power(base, exp) → integer`
///
/// Raise a GF(256) element to a non-negative integer power.
/// Uses `base^exp = ALOG[(LOG[base] * exp) mod 255]`.
/// `power(0, 0) = 1` by convention.
pub unsafe extern "C" fn nif_power(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let base = get_u8!(env, argv, 0);
    // exp is a non-negative integer (u32 range)
    let exp_term = *argv.add(1);
    let exp = match get_i64(env, exp_term) {
        Some(v) if v >= 0 => v as u32,
        _ => return badarg(env),
    };
    make_i64(env, gf256::power(base, exp) as i64)
}

/// `inverse(a) → integer`
///
/// Multiplicative inverse: `a * inverse(a) = 1`.
/// Returns `badarg` if a is 0 (zero has no inverse).
pub unsafe extern "C" fn nif_inverse(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_u8!(env, argv, 0);
    match catch_unwind(|| gf256::inverse(a)) {
        Ok(result) => make_i64(env, result as i64),
        Err(_) => badarg(env),
    }
}

// ---------------------------------------------------------------------------
// NIF function table and entry point
// ---------------------------------------------------------------------------
//
// ErlNifFunc contains raw pointer fields that are not auto-Sync.
// We wrap arrays in newtypes and assert Sync manually.
// Safety: immutable static data, read once during module load.

struct FuncTable([ErlNifFunc; 6]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    ErlNifFunc { name: b"add\0".as_ptr()      as *const _, arity: 2, fptr: nif_add,      flags: 0 },
    ErlNifFunc { name: b"subtract\0".as_ptr() as *const _, arity: 2, fptr: nif_subtract, flags: 0 },
    ErlNifFunc { name: b"multiply\0".as_ptr() as *const _, arity: 2, fptr: nif_multiply, flags: 0 },
    ErlNifFunc { name: b"divide\0".as_ptr()   as *const _, arity: 2, fptr: nif_divide,   flags: 0 },
    ErlNifFunc { name: b"power\0".as_ptr()    as *const _, arity: 2, fptr: nif_power,    flags: 0 },
    ErlNifFunc { name: b"inverse\0".as_ptr()  as *const _, arity: 1, fptr: nif_inverse,  flags: 0 },
]);

struct NifEntry(erl_nif_bridge::ErlNifEntry);
unsafe impl Sync for NifEntry {}

static MODULE_NAME_BYTES: &[u8] = b"gf256_native\0";
static VM_VARIANT_BYTES: &[u8] = b"beam.vanilla\0";
static MIN_ERTS_BYTES: &[u8] = b"erts-13.0\0";

static NIF_ENTRY: NifEntry = NifEntry(erl_nif_bridge::ErlNifEntry {
    major: erl_nif_bridge::ERL_NIF_MAJOR_VERSION,
    minor: erl_nif_bridge::ERL_NIF_MINOR_VERSION,
    name: MODULE_NAME_BYTES.as_ptr() as *const std::ffi::c_char,
    num_of_funcs: 6,
    funcs: FUNCS.0.as_ptr(),
    load: None,
    reload: None,
    upgrade: None,
    unload: None,
    vm_variant: VM_VARIANT_BYTES.as_ptr() as *const std::ffi::c_char,
    options: 0,
    sizeof_ErlNifResourceTypeInit: 0,
    min_erts: MIN_ERTS_BYTES.as_ptr() as *const std::ffi::c_char,
});

#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const erl_nif_bridge::ErlNifEntry {
    &NIF_ENTRY.0
}
