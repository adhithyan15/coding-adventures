//! # polynomial_native — Erlang NIF wrapping Rust polynomial arithmetic
//!
//! This file is the glue layer between Elixir and the Rust `polynomial` crate.
//! It uses the `erl-nif-bridge` crate to declare C-ABI functions that the BEAM
//! (Erlang's runtime) can call as Native Implemented Functions (NIFs).
//!
//! ## How a NIF works
//!
//! When Elixir calls `CodingAdventures.PolynomialNative.add([1.0, 2.0], [3.0])`,
//! the BEAM does NOT go through the bytecode interpreter. Instead it:
//! 1. Finds the `add/2` entry in the NIF function table (registered by `nif_init`).
//! 2. Calls our C-ABI function with a BEAM environment, arg count, and arg array.
//! 3. We read args, call Rust, and return a BEAM term.
//!
//! ## Polynomial representation
//!
//! Polynomials cross the boundary as **Erlang lists of floats**:
//!
//! ```text
//! Elixir: [3.0, 0.0, 1.0]  →  Rust: vec![3.0, 0.0, 1.0]  →  3 + x²
//! ```
//!
//! Index = degree: index 0 is the constant term, index n is the coefficient
//! of xⁿ. This mirrors the Rust `polynomial` crate's representation.
//!
//! ## Error handling strategy
//!
//! - Wrong argument type → `badarg` via `erl_nif_bridge::badarg(env)`
//! - Division by zero polynomial → `catch_unwind` catches the Rust panic,
//!   then we return `badarg` to Elixir
//!
//! ## Safety
//!
//! All functions are `unsafe extern "C"` because we are implementing a C ABI.
//! The actual operations (polynomial arithmetic) are entirely safe Rust inside.
//! The only unsafe code is reading from the `argv` pointer, which the BEAM
//! guarantees is valid for the duration of the NIF call.

// Allow non-snake-case names to match the NIF C API (ERL_NIF_TERM, etc.)
#![allow(non_snake_case)]

use erl_nif_bridge::{
    badarg, get_f64, get_f64_list, make_f64, make_f64_list, make_i64, ErlNifEnv,
    ErlNifFunc, ERL_NIF_TERM, enif_make_tuple_from_array,
};
use std::ffi::c_int;
use std::panic::catch_unwind;

// ---------------------------------------------------------------------------
// Helper: parse a polynomial from argv[idx], or return badarg
// ---------------------------------------------------------------------------
//
// This macro extracts a float list from the argv array and returns badarg
// on failure, keeping all the NIF functions below tidy and DRY.

macro_rules! get_poly {
    ($env:expr, $argv:expr, $idx:expr) => {{
        let term = unsafe { *$argv.add($idx) };
        match unsafe { get_f64_list($env, term) } {
            Some(v) => v,
            None => return unsafe { badarg($env) },
        }
    }};
}

macro_rules! get_scalar {
    ($env:expr, $argv:expr, $idx:expr) => {{
        let term = unsafe { *$argv.add($idx) };
        match unsafe { get_f64($env, term) } {
            Some(v) => v,
            None => return unsafe { badarg($env) },
        }
    }};
}

// ---------------------------------------------------------------------------
// NIF implementations
// ---------------------------------------------------------------------------

/// `normalize(poly) → poly`
///
/// Strip trailing near-zero coefficients. `[1.0, 0.0, 0.0]` → `[1.0]`.
pub unsafe extern "C" fn nif_normalize(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let poly = get_poly!(env, argv, 0);
    let result = polynomial::normalize(&poly);
    make_f64_list(env, &result)
}

/// `degree(poly) → integer`
///
/// Return the degree of the polynomial (highest non-zero exponent index).
/// The zero polynomial returns 0 by convention.
pub unsafe extern "C" fn nif_degree(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let poly = get_poly!(env, argv, 0);
    let d = polynomial::degree(&poly) as i64;
    make_i64(env, d)
}

/// `zero() → [0.0]`
///
/// The additive identity polynomial.
pub unsafe extern "C" fn nif_zero(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    make_f64_list(env, &polynomial::zero())
}

/// `one() → [1.0]`
///
/// The multiplicative identity polynomial.
pub unsafe extern "C" fn nif_one(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    make_f64_list(env, &polynomial::one())
}

/// `add(a, b) → poly`
///
/// Add two polynomials term-by-term. Result degree ≤ max(deg a, deg b).
pub unsafe extern "C" fn nif_add(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    make_f64_list(env, &polynomial::add(&a, &b))
}

/// `subtract(a, b) → poly`
///
/// Subtract polynomial b from a term-by-term.
pub unsafe extern "C" fn nif_subtract(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    make_f64_list(env, &polynomial::subtract(&a, &b))
}

/// `multiply(a, b) → poly`
///
/// Polynomial multiplication via convolution. Result degree = deg(a) + deg(b).
pub unsafe extern "C" fn nif_multiply(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    make_f64_list(env, &polynomial::multiply(&a, &b))
}

/// `divmod(a, b) → {quotient, remainder}`
///
/// Polynomial long division. Returns a 2-tuple `{quot, rem}` where
/// `a = b * quot + rem` and `degree(rem) < degree(b)`.
///
/// Returns `badarg` if b is the zero polynomial.
pub unsafe extern "C" fn nif_divmod(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    // Catch the panic raised by divmod when divisor is zero.
    let result = catch_unwind(|| polynomial::divmod(&a, &b));
    match result {
        Ok((q, r)) => {
            let q_term = make_f64_list(env, &q);
            let r_term = make_f64_list(env, &r);
            let arr = [q_term, r_term];
            enif_make_tuple_from_array(env, arr.as_ptr(), 2)
        }
        Err(_) => badarg(env),
    }
}

/// `divide(a, b) → poly`
///
/// Polynomial long division — quotient only. Returns `badarg` on zero divisor.
pub unsafe extern "C" fn nif_divide(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    match catch_unwind(|| polynomial::divide(&a, &b)) {
        Ok(q) => make_f64_list(env, &q),
        Err(_) => badarg(env),
    }
}

/// `modulo(a, b) → poly`
///
/// Polynomial long division — remainder only. Returns `badarg` on zero divisor.
pub unsafe extern "C" fn nif_modulo(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    match catch_unwind(|| polynomial::modulo(&a, &b)) {
        Ok(r) => make_f64_list(env, &r),
        Err(_) => badarg(env),
    }
}

/// `evaluate(poly, x) → float`
///
/// Evaluate the polynomial at the point x using Horner's method.
pub unsafe extern "C" fn nif_evaluate(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let poly = get_poly!(env, argv, 0);
    let x = get_scalar!(env, argv, 1);
    make_f64(env, polynomial::evaluate(&poly, x))
}

/// `gcd(a, b) → poly`
///
/// Greatest common divisor of two polynomials via the Euclidean algorithm.
pub unsafe extern "C" fn nif_gcd(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let a = get_poly!(env, argv, 0);
    let b = get_poly!(env, argv, 1);
    make_f64_list(env, &polynomial::gcd(&a, &b))
}

// ---------------------------------------------------------------------------
// NIF function table — tells the BEAM which Rust functions to call
// ---------------------------------------------------------------------------
//
// Each `ErlNifFunc` entry maps an Erlang function name+arity to a C function.
// The BEAM uses this table when it resolves calls like `polynomial_native:add/2`.

// ---------------------------------------------------------------------------
// NIF function table and entry point
// ---------------------------------------------------------------------------
//
// ErlNifFunc and ErlNifEntry contain raw pointer fields (*const c_char) that
// are not automatically Sync. Since `static` variables require Sync, we wrap
// them in newtypes and assert Sync manually.
//
// Safety rationale: These are immutable, constant data segments. The BEAM
// reads them once during module load (effectively single-threaded startup).
// There is no concurrent mutation — the `unsafe impl Sync` is sound here.

/// Wrapper making ErlNifFunc slices usable as `static` values.
struct FuncTable([ErlNifFunc; 12]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    ErlNifFunc { name: b"normalize\0".as_ptr() as *const _, arity: 1, fptr: nif_normalize, flags: 0 },
    ErlNifFunc { name: b"degree\0".as_ptr()    as *const _, arity: 1, fptr: nif_degree,    flags: 0 },
    ErlNifFunc { name: b"zero\0".as_ptr()      as *const _, arity: 0, fptr: nif_zero,      flags: 0 },
    ErlNifFunc { name: b"one\0".as_ptr()       as *const _, arity: 0, fptr: nif_one,       flags: 0 },
    ErlNifFunc { name: b"add\0".as_ptr()       as *const _, arity: 2, fptr: nif_add,       flags: 0 },
    ErlNifFunc { name: b"subtract\0".as_ptr()  as *const _, arity: 2, fptr: nif_subtract,  flags: 0 },
    ErlNifFunc { name: b"multiply\0".as_ptr()  as *const _, arity: 2, fptr: nif_multiply,  flags: 0 },
    ErlNifFunc { name: b"divmod\0".as_ptr()    as *const _, arity: 2, fptr: nif_divmod,    flags: 0 },
    ErlNifFunc { name: b"divide\0".as_ptr()    as *const _, arity: 2, fptr: nif_divide,    flags: 0 },
    ErlNifFunc { name: b"modulo\0".as_ptr()    as *const _, arity: 2, fptr: nif_modulo,    flags: 0 },
    ErlNifFunc { name: b"evaluate\0".as_ptr()  as *const _, arity: 2, fptr: nif_evaluate,  flags: 0 },
    ErlNifFunc { name: b"gcd\0".as_ptr()       as *const _, arity: 2, fptr: nif_gcd,       flags: 0 },
]);

/// Wrapper making ErlNifEntry usable as a `static` value.
struct NifEntry(erl_nif_bridge::ErlNifEntry);
unsafe impl Sync for NifEntry {}

static MODULE_NAME_BYTES: &[u8] = b"polynomial_native\0";
static VM_VARIANT_BYTES: &[u8] = b"beam.vanilla\0";
static MIN_ERTS_BYTES: &[u8] = b"erts-13.0\0";

static NIF_ENTRY: NifEntry = NifEntry(erl_nif_bridge::ErlNifEntry {
    major: erl_nif_bridge::ERL_NIF_MAJOR_VERSION,
    minor: erl_nif_bridge::ERL_NIF_MINOR_VERSION,
    name: MODULE_NAME_BYTES.as_ptr() as *const std::ffi::c_char,
    num_of_funcs: 12, // must match FUNCS array length
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

/// NIF library entry point — called by the BEAM on `:erlang.load_nif/2`.
///
/// Returns a pointer to the static module descriptor. The BEAM reads the
/// function table from it to bind Erlang function names to our C functions.
///
/// # Safety
/// The returned pointer is valid for the lifetime of the process.
#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const erl_nif_bridge::ErlNifEntry {
    &NIF_ENTRY.0
}
