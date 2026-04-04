// lib.rs -- GF(2^8) Node.js native extension using node-bridge
// =============================================================
//
// This crate exposes the Rust `gf256` crate to Node.js via N-API,
// using our zero-dependency `node-bridge` crate. No napi-rs, no napi-sys,
// no build-time header requirements -- just raw N-API calls through
// node-bridge's safe wrappers.
//
// # What is GF(2^8)?
//
// GF(2^8) -- "Galois Field of 256 elements" -- is the finite field used in:
//   - Reed-Solomon error correction (QR codes, CDs, DVDs)
//   - AES encryption (SubBytes and MixColumns steps)
//
// Elements are bytes (0..=255). Addition is XOR. Multiplication uses
// precomputed logarithm tables and reduces modulo the primitive polynomial
// x^8 + x^4 + x^3 + x^2 + 1 (= 0x11D = 285).
//
// # Architecture
//
// Like polynomial-native, this module exposes pure free functions.
// Each function:
//   1. Extracts numeric arguments from JS (as u32, then casts to u8).
//   2. Calls the corresponding Rust gf256 function.
//   3. Converts the u8 result back to a JS number.
//
// GF(256) elements are bytes (0..255), but JS has no u8 type -- all numbers
// are f64. We accept JS numbers as u32 (via napi_get_value_uint32), cast
// them to u8, and return results as i64 (via napi_create_int64).
//
// Division by zero and inverse of zero cause Rust panics; we catch them with
// `std::panic::catch_unwind` and convert to JS exceptions.
//
// # Module constants
//
// We also expose the three constants from the Rust crate as properties on
// the exports object:
//   ZERO               = 0   (additive identity)
//   ONE                = 1   (multiplicative identity)
//   PRIMITIVE_POLYNOMIAL = 285 (= 0x11D, the irreducible polynomial)
//
// These are set using `napi_set_named_property` + `napi_create_int32`.

use node_bridge::*;
use std::ptr;

// ---------------------------------------------------------------------------
// Extra N-API externs not in node-bridge
// ---------------------------------------------------------------------------
//
// We need `napi_get_value_uint32` to extract u8-range numbers from JS
// (u32 because JS numbers are f64, and bitwise u8 values are all < 256),
// and `napi_create_int32` for the module constants.

extern "C" {
    fn napi_get_value_uint32(
        env: napi_env,
        value: napi_value,
        result: *mut u32,
    ) -> napi_status;

    fn napi_create_int32(
        env: napi_env,
        value: i32,
        result: *mut napi_value,
    ) -> napi_status;
}

// ---------------------------------------------------------------------------
// Helper: extract a JS number as u8 (via u32)
// ---------------------------------------------------------------------------
//
// GF(256) elements are bytes in [0, 255]. JS represents them as f64, but
// `napi_get_value_uint32` gives us a clean u32. We then cast to u8,
// silently truncating values >= 256 (the caller is responsible for valid input).

fn u8_from_js(env: napi_env, val: napi_value) -> u8 {
    let mut result: u32 = 0;
    unsafe { napi_get_value_uint32(env, val, &mut result) };
    result as u8
}

// ---------------------------------------------------------------------------
// Helper: extract a JS number as u32 (for the `exp` argument of `power`)
// ---------------------------------------------------------------------------

fn u32_from_js(env: napi_env, val: napi_value) -> u32 {
    let mut result: u32 = 0;
    unsafe { napi_get_value_uint32(env, val, &mut result) };
    result
}

// ---------------------------------------------------------------------------
// Helper: convert a u8 result to a JS number
// ---------------------------------------------------------------------------

fn u8_to_js(env: napi_env, val: u8) -> napi_value {
    usize_to_js(env, val as usize)
}

// ---------------------------------------------------------------------------
// Helper: create a JS integer constant
// ---------------------------------------------------------------------------

fn int32_to_js(env: napi_env, val: i32) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    unsafe { napi_create_int32(env, val, &mut result) };
    result
}

// ---------------------------------------------------------------------------
// Panic-catching helper
// ---------------------------------------------------------------------------
//
// `gf256::divide` panics with "GF256: division by zero" when b == 0.
// `gf256::inverse` panics with "GF256: zero has no multiplicative inverse" when a == 0.
// We catch these panics and re-raise them as JS exceptions.

fn run_or_throw<F, T>(env: napi_env, f: F) -> Option<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(val) => Some(val),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "GF256 operation panicked".to_string()
            };
            throw_error(env, &msg);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// add(a: number, b: number) -> number
// ---------------------------------------------------------------------------
//
// GF(256) addition is bitwise XOR.
//
// ```
// add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99 = 153
// add(x, x) = 0  for all x  (every element is its own additive inverse)
// ```

unsafe extern "C" fn gf_add(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "add requires two GF256 element arguments");
        return undefined(env);
    }
    let a = u8_from_js(env, args[0]);
    let b = u8_from_js(env, args[1]);
    u8_to_js(env, gf256::add(a, b))
}

// ---------------------------------------------------------------------------
// subtract(a: number, b: number) -> number
// ---------------------------------------------------------------------------
//
// In characteristic-2 fields, subtraction equals addition (XOR).
// This is not a coincidence: -1 = 1 in GF(2), so -b = b.

unsafe extern "C" fn gf_subtract(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "subtract requires two GF256 element arguments");
        return undefined(env);
    }
    let a = u8_from_js(env, args[0]);
    let b = u8_from_js(env, args[1]);
    u8_to_js(env, gf256::subtract(a, b))
}

// ---------------------------------------------------------------------------
// multiply(a: number, b: number) -> number
// ---------------------------------------------------------------------------
//
// GF(256) multiplication using logarithm/antilogarithm tables:
//   a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
//
// Special case: anything × 0 = 0 (zero is not in the multiplicative group).

unsafe extern "C" fn gf_multiply(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "multiply requires two GF256 element arguments");
        return undefined(env);
    }
    let a = u8_from_js(env, args[0]);
    let b = u8_from_js(env, args[1]);
    u8_to_js(env, gf256::multiply(a, b))
}

// ---------------------------------------------------------------------------
// divide(a: number, b: number) -> number
// ---------------------------------------------------------------------------
//
// GF(256) division: a / b = ALOG[(LOG[a] - LOG[b] + 255) mod 255]
//
// Special case: 0 / b = 0 for any non-zero b.
// Throws if b == 0 (division by zero is undefined in any field).

unsafe extern "C" fn gf_divide(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "divide requires two GF256 element arguments");
        return undefined(env);
    }
    let a = u8_from_js(env, args[0]);
    let b = u8_from_js(env, args[1]);
    match run_or_throw(env, move || gf256::divide(a, b)) {
        None => undefined(env),
        Some(result) => u8_to_js(env, result),
    }
}

// ---------------------------------------------------------------------------
// power(base: number, exp: number) -> number
// ---------------------------------------------------------------------------
//
// GF(256) exponentiation: base^exp = ALOG[(LOG[base] * exp) mod 255]
//
// Special cases:
//   0^0 = 1  (by convention, consistent with most math libraries)
//   0^n = 0  for n > 0
//   b^0 = 1  for any b (the empty product)
//
// The `exp` argument is a u32 (not u8) because exponents in Reed-Solomon
// can exceed 255. The modular group order (255) handles wrap-around.

unsafe extern "C" fn gf_power(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "power requires (base, exp) arguments");
        return undefined(env);
    }
    let base = u8_from_js(env, args[0]);
    let exp = u32_from_js(env, args[1]);
    u8_to_js(env, gf256::power(base, exp))
}

// ---------------------------------------------------------------------------
// inverse(a: number) -> number
// ---------------------------------------------------------------------------
//
// GF(256) multiplicative inverse: a^(-1) = ALOG[255 - LOG[a]]
//
// Satisfies: a × inverse(a) = 1 for all non-zero a.
// Throws if a == 0 (zero has no multiplicative inverse).

unsafe extern "C" fn gf_inverse(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "inverse requires one GF256 element argument");
        return undefined(env);
    }
    let a = u8_from_js(env, args[0]);
    match run_or_throw(env, move || gf256::inverse(a)) {
        None => undefined(env),
        Some(result) => u8_to_js(env, result),
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We expose all GF(256) operations as free functions AND the three constants
// (ZERO, ONE, PRIMITIVE_POLYNOMIAL) as properties on the exports object.
//
// Constants are set first, then functions. Both use `set_named_property`.

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // -----------------------------------------------------------------------
    // Constants
    // -----------------------------------------------------------------------
    //
    // ZERO = 0 (additive identity; XOR with anything leaves it unchanged)
    // ONE  = 1 (multiplicative identity; multiply by 1 leaves it unchanged)
    // PRIMITIVE_POLYNOMIAL = 285 = 0x11D
    //   (the irreducible polynomial x^8 + x^4 + x^3 + x^2 + 1 used for
    //    modular reduction in multiplication)

    set_named_property(env, exports, "ZERO",
        int32_to_js(env, gf256::ZERO as i32));

    set_named_property(env, exports, "ONE",
        int32_to_js(env, gf256::ONE as i32));

    set_named_property(env, exports, "PRIMITIVE_POLYNOMIAL",
        int32_to_js(env, gf256::PRIMITIVE_POLYNOMIAL as i32));

    // -----------------------------------------------------------------------
    // Free functions
    // -----------------------------------------------------------------------

    set_named_property(env, exports, "add",
        create_function(env, "add", Some(gf_add)));

    set_named_property(env, exports, "subtract",
        create_function(env, "subtract", Some(gf_subtract)));

    set_named_property(env, exports, "multiply",
        create_function(env, "multiply", Some(gf_multiply)));

    set_named_property(env, exports, "divide",
        create_function(env, "divide", Some(gf_divide)));

    set_named_property(env, exports, "power",
        create_function(env, "power", Some(gf_power)));

    set_named_property(env, exports, "inverse",
        create_function(env, "inverse", Some(gf_inverse)));

    exports
}
