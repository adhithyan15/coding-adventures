//! # polynomial-c — C ABI wrapper for the `polynomial` crate.
//!
//! This crate exposes the polynomial arithmetic functions from the `polynomial`
//! crate over a **stable C ABI**. Swift, C, and C++ callers can link against the
//! compiled static library (`libpolynomial_c.a`) and call these functions directly
//! at compile time — no runtime FFI bridge, no boxing, no dynamic loading.
//!
//! ## Why a Separate Crate?
//!
//! The `polynomial` crate uses idiomatic Rust slices (`&[f64]`), `Vec<f64>`,
//! and panics for errors. None of these concepts exist in C's type system.
//! This wrapper crate:
//!
//! 1. Converts raw pointer + length pairs into safe Rust slices.
//! 2. Writes results into caller-provided output buffers (avoiding heap allocation
//!    across the FFI boundary — allocating in Rust and freeing in Swift is unsafe).
//! 3. Catches Rust panics via `std::panic::catch_unwind` and returns error codes
//!    so that a panic in the Rust code does not unwind through C frames (which is
//!    undefined behaviour).
//!
//! ## Memory Protocol
//!
//! **The caller owns all memory.** This is the universal rule for safe C FFI.
//!
//! Array-returning functions use a buffer protocol:
//! - The caller allocates a buffer with enough capacity for the worst-case result.
//! - The function writes the result into that buffer.
//! - The function returns the number of elements actually written.
//!
//! Worst-case sizes:
//!
//! | Operation | Worst-case output length |
//! |-----------|--------------------------|
//! | normalize | `input_len` (no growth)  |
//! | add       | `max(a_len, b_len)`      |
//! | subtract  | `max(a_len, b_len)`      |
//! | multiply  | `a_len + b_len - 1`      |
//! | divide    | `a_len` (quotient ≤ dividend length) |
//! | modulo    | `b_len` (remainder < divisor) |
//! | gcd       | `max(a_len, b_len)`      |
//! | divmod    | quotient + remainder (two buffers) |
//!
//! ## Safety Contract
//!
//! All functions are marked `unsafe` because they dereference raw pointers.
//! The caller must guarantee:
//! - `coeffs` / `a` / `b` point to valid, aligned, initialised `f64` arrays.
//! - `out` points to a writable buffer of at least `out_cap` `f64` elements.
//! - No aliasing between input and output buffers.
//! - Pointer + length describe the same allocation (no out-of-bounds access).

use std::ffi::c_int;
use std::panic;

// =============================================================================
// Helper: slice from raw parts
// =============================================================================

/// Build a `&[f64]` from a raw pointer and a length.
///
/// This is a thin helper so every exported function uses the same safe
/// idiom rather than repeating the unsafe call to `from_raw_parts`.
///
/// # Safety
///
/// The caller must guarantee `ptr` is non-null and points to `len`
/// initialised `f64` values that are valid for the duration of this call.
#[inline]
unsafe fn slice<'a>(ptr: *const f64, len: usize) -> &'a [f64] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: guaranteed by the caller's contract.
        std::slice::from_raw_parts(ptr, len)
    }
}

/// Write a `Vec<f64>` result into a caller-provided output buffer.
///
/// Copies at most `out_cap` elements and returns the number written.
/// If the result is longer than `out_cap`, it is silently truncated.
/// (Callers allocating worst-case buffers will never truncate.)
#[inline]
unsafe fn write_out(result: Vec<f64>, out: *mut f64, out_cap: usize) -> usize {
    let n = result.len().min(out_cap);
    if n > 0 && !out.is_null() {
        // SAFETY: guaranteed by the caller's contract.
        std::ptr::copy_nonoverlapping(result.as_ptr(), out, n);
    }
    n
}

// =============================================================================
// Fundamentals
// =============================================================================

/// Normalize a polynomial (strip trailing near-zero coefficients).
///
/// Returns the number of elements written to `out`.
///
/// # Example (from C)
///
/// ```c
/// double poly[]   = {1.0, 0.0, 0.0};
/// double out[4];
/// size_t n = poly_c_normalize(poly, 3, out, 4);
/// // n == 1, out[0] == 1.0
/// ```
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_normalize(
    coeffs: *const f64,
    len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    // Catch any Rust panics to avoid unwinding through C frames (UB).
    match panic::catch_unwind(|| {
        let poly = unsafe { slice(coeffs, len) };
        let result = polynomial::normalize(poly);
        unsafe { write_out(result, out, out_cap) }
    }) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

/// Return the degree of a polynomial.
///
/// The degree is the index of the highest non-zero coefficient.
/// The zero polynomial returns 0 by convention.
///
/// Returns `usize::MAX` as a sentinel on panic (which should never happen in
/// practice, but panicking across a C ABI boundary is undefined behaviour).
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_degree(coeffs: *const f64, len: usize) -> usize {
    // Clone the slice data so the closure is 'static (no borrow crossing the
    // catch_unwind boundary).
    let owned: Vec<f64> = slice(coeffs, len).to_vec();
    match panic::catch_unwind(move || polynomial::degree(&owned)) {
        Ok(d) => d,
        Err(_) => usize::MAX, // sentinel: degree cannot be usize::MAX in practice
    }
}

/// Evaluate a polynomial at `x` using Horner's method.
///
/// The zero polynomial evaluates to 0.0.
///
/// Returns `f64::NAN` as a conventional error sentinel on panic (which should
/// never happen in practice, but panicking across a C ABI boundary is
/// undefined behaviour).
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_evaluate(coeffs: *const f64, len: usize, x: f64) -> f64 {
    // Clone the slice data so the closure is 'static (no borrow crossing the
    // catch_unwind boundary).
    let owned: Vec<f64> = slice(coeffs, len).to_vec();
    match panic::catch_unwind(move || polynomial::evaluate(&owned, x)) {
        Ok(v) => v,
        Err(_) => f64::NAN, // NaN is the conventional floating-point error sentinel
    }
}

// =============================================================================
// Addition and Subtraction
// =============================================================================

/// Add two polynomials term-by-term.
///
/// Worst-case output length: `max(a_len, b_len)`.
///
/// Returns the number of elements written to `out`.
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_add(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    // Catch any Rust panics to avoid unwinding through C frames (UB).
    match panic::catch_unwind(|| {
        let sa = unsafe { slice(a, a_len) };
        let sb = unsafe { slice(b, b_len) };
        let result = polynomial::add(sa, sb);
        unsafe { write_out(result, out, out_cap) }
    }) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

/// Subtract polynomial `b` from polynomial `a` term-by-term.
///
/// Worst-case output length: `max(a_len, b_len)`.
///
/// Returns the number of elements written to `out`.
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_subtract(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    // Catch any Rust panics to avoid unwinding through C frames (UB).
    match panic::catch_unwind(|| {
        let sa = unsafe { slice(a, a_len) };
        let sb = unsafe { slice(b, b_len) };
        let result = polynomial::subtract(sa, sb);
        unsafe { write_out(result, out, out_cap) }
    }) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

// =============================================================================
// Multiplication
// =============================================================================

/// Multiply two polynomials by polynomial convolution.
///
/// Worst-case output length: `a_len + b_len - 1` (or 0 if either is empty).
///
/// Returns the number of elements written to `out`.
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_multiply(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    // Catch any Rust panics to avoid unwinding through C frames (UB).
    match panic::catch_unwind(|| {
        let sa = unsafe { slice(a, a_len) };
        let sb = unsafe { slice(b, b_len) };
        let result = polynomial::multiply(sa, sb);
        unsafe { write_out(result, out, out_cap) }
    }) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

// =============================================================================
// Division
// =============================================================================

/// Perform polynomial division and modulo simultaneously.
///
/// Finds `(quotient, remainder)` such that:
///
/// ```text
/// dividend = divisor × quotient + remainder
/// degree(remainder) < degree(divisor)
/// ```
///
/// The result is written into two caller-provided buffers:
/// - `quot_out` / `quot_cap` / `quot_len_out` — the quotient.
/// - `rem_out`  / `rem_cap`  / `rem_len_out`  — the remainder.
///
/// Returns 0 on success, -1 if the divisor is the zero polynomial (which
/// would cause division by zero). On error, no output buffers are written.
///
/// # Worst-Case Buffer Sizes
///
/// | Buffer       | Worst-case size |
/// |--------------|-----------------|
/// | `quot_out`   | `dividend_len`  |
/// | `rem_out`    | `divisor_len`   |
///
/// # Safety
///
/// See module-level safety contract. Both output pointer/length pairs must
/// be valid if the return value is 0 (success).
#[no_mangle]
pub unsafe extern "C" fn poly_c_divmod(
    dividend: *const f64,
    dividend_len: usize,
    divisor: *const f64,
    divisor_len: usize,
    quot_out: *mut f64,
    quot_cap: usize,
    quot_len_out: *mut usize,
    rem_out: *mut f64,
    rem_cap: usize,
    rem_len_out: *mut usize,
) -> c_int {
    // Clone slice data into owned Vecs before entering catch_unwind.
    // This eliminates the borrow-in-closure pattern: the move closure owns
    // the data, so there is no lifetime or aliasing ambiguity.
    let sd_owned: Vec<f64> = slice(dividend, dividend_len).to_vec();
    let sb_owned: Vec<f64> = slice(divisor, divisor_len).to_vec();

    // Catch any Rust panics (e.g., division by zero in the underlying crate)
    // and translate them to an error code instead of unwinding through C frames.
    // Unwinding through C frames is undefined behaviour in Rust.
    let result = panic::catch_unwind(move || polynomial::divmod(&sd_owned, &sb_owned));

    match result {
        Ok((quot, rem)) => {
            let qn = write_out(quot, quot_out, quot_cap);
            let rn = write_out(rem, rem_out, rem_cap);
            if !quot_len_out.is_null() {
                *quot_len_out = qn;
            }
            if !rem_len_out.is_null() {
                *rem_len_out = rn;
            }
            0 // success
        }
        Err(_) => -1, // panic (division by zero polynomial)
    }
}

/// Return the quotient of `dividend / divisor`.
///
/// Worst-case output length: `dividend_len`.
///
/// Returns the number of elements written to `out`, or 0 on error (zero
/// divisor). Unlike `poly_c_divmod`, callers cannot distinguish a zero
/// quotient from an error; use `poly_c_divmod` if error detection matters.
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_divide(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    let sa = slice(a, a_len);
    let sb = slice(b, b_len);
    match panic::catch_unwind(|| polynomial::divide(sa, sb)) {
        Ok(result) => write_out(result, out, out_cap),
        Err(_) => 0,
    }
}

/// Return the remainder of `dividend / divisor`.
///
/// Worst-case output length: `divisor_len` (remainder degree < divisor degree).
///
/// Returns the number of elements written to `out`, or 0 on error (zero
/// divisor).
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_modulo(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    let sa = slice(a, a_len);
    let sb = slice(b, b_len);
    match panic::catch_unwind(|| polynomial::modulo(sa, sb)) {
        Ok(result) => write_out(result, out, out_cap),
        Err(_) => 0,
    }
}

// =============================================================================
// Greatest Common Divisor
// =============================================================================

/// Compute the GCD of two polynomials using the Euclidean algorithm.
///
/// Worst-case output length: `max(a_len, b_len)`.
///
/// Returns the number of elements written to `out`.
///
/// # Safety
///
/// See module-level safety contract.
#[no_mangle]
pub unsafe extern "C" fn poly_c_gcd(
    a: *const f64,
    a_len: usize,
    b: *const f64,
    b_len: usize,
    out: *mut f64,
    out_cap: usize,
) -> usize {
    // Catch any Rust panics to avoid unwinding through C frames (UB).
    match panic::catch_unwind(|| {
        let sa = unsafe { slice(a, a_len) };
        let sb = unsafe { slice(b, b_len) };
        let result = polynomial::gcd(sa, sb);
        unsafe { write_out(result, out, out_cap) }
    }) {
        Ok(n) => n,
        Err(_) => 0,
    }
}
