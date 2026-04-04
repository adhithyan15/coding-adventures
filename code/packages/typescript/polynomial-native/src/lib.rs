// lib.rs -- Polynomial Node.js native extension using node-bridge
// ===============================================================
//
// This crate exposes the Rust `polynomial` crate to Node.js via N-API,
// using our zero-dependency `node-bridge` crate. No napi-rs, no napi-sys,
// no build-time header requirements -- just raw N-API calls through
// node-bridge's safe wrappers.
//
// # Architecture
//
// Unlike bitset-native (which wraps a stateful object), polynomial-native
// exposes a set of pure free functions. Every function:
//   1. Reads JS array argument(s) as Vec<f64>.
//   2. Calls the corresponding Rust polynomial function.
//   3. Converts the Vec<f64> result back to a JS array.
//
// Because polynomial::divmod can panic on zero divisor, and polynomial
// arithmetic can internally panic in degenerate cases, all calls are
// wrapped with `std::panic::catch_unwind` to turn panics into JS exceptions.
//
// # Array convention
//
// JS arrays of numbers represent polynomial coefficients in "little-endian"
// order: index 0 = constant term (degree 0), index 1 = x^1 coefficient, etc.
// This matches the Rust `polynomial` crate's convention exactly.
//
// # Function naming
//
// All functions use camelCase to follow JavaScript conventions:
//   normalize, degree, zero, one,
//   add, subtract, multiply,
//   divmodPoly (avoid conflict with "divmod" keyword ambiguity),
//   divide, modulo (not "mod" -- reserved in Rust),
//   evaluate, gcd

use node_bridge::*;
use std::ptr;

// ---------------------------------------------------------------------------
// Extra N-API externs not in node-bridge
// ---------------------------------------------------------------------------
//
// We need `napi_get_value_double` and `napi_create_double` to marshal f64
// values between Rust and JavaScript. node-bridge only has integer support
// (napi_create_int64 / napi_get_value_int64), so we declare the double
// variants here.

extern "C" {
    fn napi_get_value_double(
        env: napi_env,
        value: napi_value,
        result: *mut f64,
    ) -> napi_status;

    fn napi_create_double(
        env: napi_env,
        value: f64,
        result: *mut napi_value,
    ) -> napi_status;

    fn napi_create_array_with_length(
        env: napi_env,
        length: usize,
        result: *mut napi_value,
    ) -> napi_status;
}

// ---------------------------------------------------------------------------
// Low-level helpers: f64 <-> JS number
// ---------------------------------------------------------------------------

/// Convert a JS number value to an f64.
///
/// N-API uses f64 (IEEE 754 double) for all JS numbers internally.
/// `napi_get_value_double` extracts this representation.
///
/// Returns None and throws a JS exception if the value is not a JS number
/// (e.g., a string or object was passed where a number was expected).
fn f64_from_js(env: napi_env, val: napi_value) -> Option<f64> {
    let mut result: f64 = 0.0;
    let status = unsafe { napi_get_value_double(env, val, &mut result) };
    if status != 0 {
        // napi_ok == 0; any other status means the value is not a number.
        throw_error(env, "expected a number");
        return None;
    }
    Some(result)
}

/// Convert an f64 to a JS number value.
///
/// `napi_create_double` creates a JS Number from an f64.
/// Returns undefined and throws a JS exception if N-API fails to create the value.
fn f64_to_js(env: napi_env, val: f64) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_create_double(env, val, &mut result) };
    if status != 0 {
        throw_error(env, "failed to create JS number");
        return undefined(env);
    }
    result
}

/// Create a JS array with a pre-allocated length (more efficient than
/// `napi_create_array` when the length is known upfront).
fn array_with_length(env: napi_env, length: usize) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    unsafe { napi_create_array_with_length(env, length, &mut result) };
    result
}

// ---------------------------------------------------------------------------
// Conversion helpers: Vec<f64> <-> JS number array
// ---------------------------------------------------------------------------

/// Convert a JS array of numbers to a Rust Vec<f64>.
///
/// Each element is extracted as a double. Returns None (with a pending JS
/// exception) if any element fails to convert (e.g., a non-number value in
/// the array). Returns Some(empty Vec) for an empty array.
///
/// ## Memory layout
///
/// JS arrays store values as napi_value pointers (opaque handles). We
/// loop over indices 0..len, call `napi_get_element` for each, then
/// `napi_get_value_double` to extract the f64 value.
fn vec_f64_from_js(env: napi_env, arr: napi_value) -> Option<Vec<f64>> {
    let len = array_len(env, arr) as usize;
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let elem = array_get(env, arr, i as u32);
        result.push(f64_from_js(env, elem)?);
    }
    Some(result)
}

/// Convert a Rust Vec<f64> to a JS array of numbers.
///
/// We use `napi_create_array_with_length` rather than `napi_create_array`
/// to pre-allocate the exact capacity needed, which avoids internal
/// re-allocation in V8.
fn vec_f64_to_js(env: napi_env, v: &[f64]) -> napi_value {
    let arr = array_with_length(env, v.len());
    for (i, &coeff) in v.iter().enumerate() {
        array_set(env, arr, i as u32, f64_to_js(env, coeff));
    }
    arr
}

// ---------------------------------------------------------------------------
// Panic-catching helper
// ---------------------------------------------------------------------------
//
// The polynomial crate panics when:
//   - divmod is called with a zero divisor ("polynomial division by zero")
//
// Rather than letting the panic unwind through N-API (which would crash
// the Node.js process), we catch it and re-raise it as a JS exception.

fn run_or_throw<F, T>(env: napi_env, f: F) -> Option<T>
where
    F: FnOnce() -> T + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(val) => Some(val),
        Err(e) => {
            // The panic payload is usually a &str or String.
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "polynomial operation panicked".to_string()
            };
            throw_error(env, &msg);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// normalize(poly: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Strips trailing near-zero coefficients from `poly`.
//
// ## Example
//
//   normalize([1.0, 0.0, 0.0])  →  [1.0]
//   normalize([0.0])             →  []
//   normalize([1.0, 2.0, 3.0])  →  [1.0, 2.0, 3.0]

unsafe extern "C" fn poly_normalize(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "normalize requires a poly argument");
        return undefined(env);
    }
    let poly = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::normalize(&poly);
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// degree(poly: number[]) -> number
// ---------------------------------------------------------------------------
//
// Returns the degree of the polynomial (index of highest non-zero coefficient).
// Returns 0 for the zero polynomial.

unsafe extern "C" fn poly_degree(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "degree requires a poly argument");
        return undefined(env);
    }
    let poly = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let deg = polynomial::degree(&poly);
    usize_to_js(env, deg)
}

// ---------------------------------------------------------------------------
// zero() -> number[]
// ---------------------------------------------------------------------------
//
// Returns the zero polynomial [0.0] -- the additive identity.

unsafe extern "C" fn poly_zero(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, _args) = get_cb_info(env, info, 0);
    let result = polynomial::zero();
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// one() -> number[]
// ---------------------------------------------------------------------------
//
// Returns the one polynomial [1.0] -- the multiplicative identity.

unsafe extern "C" fn poly_one(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, _args) = get_cb_info(env, info, 0);
    let result = polynomial::one();
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// add(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Adds two polynomials term-by-term.

unsafe extern "C" fn poly_add(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "add requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::add(&a, &b);
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// subtract(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Subtracts polynomial b from polynomial a term-by-term.

unsafe extern "C" fn poly_subtract(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "subtract requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::subtract(&a, &b);
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// multiply(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Multiplies two polynomials using convolution.

unsafe extern "C" fn poly_multiply(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "multiply requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::multiply(&a, &b);
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// divmodPoly(dividend: number[], divisor: number[]) -> [number[], number[]]
// ---------------------------------------------------------------------------
//
// Performs polynomial long division, returning [quotient, remainder] as a
// 2-element JS array. Each element is itself a number[] (coefficient array).
//
// Throws a JS exception if divisor is the zero polynomial, because the Rust
// `polynomial::divmod` panics in that case -- we catch the panic via
// `std::panic::catch_unwind` and convert it to a proper JS error.
//
// ## Return value layout
//
//   result[0] = quotient  (number[])
//   result[1] = remainder (number[])

unsafe extern "C" fn poly_divmod(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "divmodPoly requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };

    // polynomial::divmod panics on zero divisor -- catch and re-throw as JS error.
    let pair = run_or_throw(env, move || polynomial::divmod(&a, &b));
    match pair {
        None => undefined(env), // exception already thrown
        Some((quot, rem)) => {
            // Build outer array of length 2.
            let outer = array_with_length(env, 2);
            let quot_js = vec_f64_to_js(env, &quot);
            let rem_js = vec_f64_to_js(env, &rem);
            array_set(env, outer, 0, quot_js);
            array_set(env, outer, 1, rem_js);
            outer
        }
    }
}

// ---------------------------------------------------------------------------
// divide(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Returns the quotient of polynomial division (divmod(a, b).0).
// Throws if b is the zero polynomial.

unsafe extern "C" fn poly_divide(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "divide requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    match run_or_throw(env, move || polynomial::divide(&a, &b)) {
        None => undefined(env),
        Some(result) => vec_f64_to_js(env, &result),
    }
}

// ---------------------------------------------------------------------------
// modulo(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Returns the remainder of polynomial division (divmod(a, b).1).
// Named "modulo" rather than "mod" because "mod" is a reserved keyword in JS.
// Throws if b is the zero polynomial.

unsafe extern "C" fn poly_modulo(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "modulo requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    match run_or_throw(env, move || polynomial::modulo(&a, &b)) {
        None => undefined(env),
        Some(result) => vec_f64_to_js(env, &result),
    }
}

// ---------------------------------------------------------------------------
// evaluate(poly: number[], x: number) -> number
// ---------------------------------------------------------------------------
//
// Evaluates a polynomial at x using Horner's method (O(n) time, no powers).

unsafe extern "C" fn poly_evaluate(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "evaluate requires (poly, x) arguments");
        return undefined(env);
    }
    let poly = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let x = match f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::evaluate(&poly, x);
    f64_to_js(env, result)
}

// ---------------------------------------------------------------------------
// gcd(a: number[], b: number[]) -> number[]
// ---------------------------------------------------------------------------
//
// Returns the greatest common divisor of two polynomials using the
// Euclidean algorithm. The result is the highest-degree polynomial
// that divides both inputs with zero remainder.

unsafe extern "C" fn poly_gcd(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "gcd requires two polynomial arguments");
        return undefined(env);
    }
    let a = match vec_f64_from_js(env, args[0]) { Some(v) => v, None => return undefined(env) };
    let b = match vec_f64_from_js(env, args[1]) { Some(v) => v, None => return undefined(env) };
    let result = polynomial::gcd(&a, &b);
    vec_f64_to_js(env, &result)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We expose all polynomial operations as free functions (not a class)
// attached directly to the exports object.

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Create and register each free function on the exports object.
    // We use node-bridge's `create_function` + `set_named_property` helpers.

    set_named_property(env, exports, "normalize",
        create_function(env, "normalize", Some(poly_normalize)));

    set_named_property(env, exports, "degree",
        create_function(env, "degree", Some(poly_degree)));

    set_named_property(env, exports, "zero",
        create_function(env, "zero", Some(poly_zero)));

    set_named_property(env, exports, "one",
        create_function(env, "one", Some(poly_one)));

    set_named_property(env, exports, "add",
        create_function(env, "add", Some(poly_add)));

    set_named_property(env, exports, "subtract",
        create_function(env, "subtract", Some(poly_subtract)));

    set_named_property(env, exports, "multiply",
        create_function(env, "multiply", Some(poly_multiply)));

    set_named_property(env, exports, "divmodPoly",
        create_function(env, "divmodPoly", Some(poly_divmod)));

    set_named_property(env, exports, "divide",
        create_function(env, "divide", Some(poly_divide)));

    set_named_property(env, exports, "modulo",
        create_function(env, "modulo", Some(poly_modulo)));

    set_named_property(env, exports, "evaluate",
        create_function(env, "evaluate", Some(poly_evaluate)));

    set_named_property(env, exports, "gcd",
        create_function(env, "gcd", Some(poly_gcd)));

    exports
}
