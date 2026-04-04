//! # gf256-c — C ABI wrapper for the `gf256` crate.
//!
//! This crate exposes GF(2^8) arithmetic over a stable C ABI so that Swift,
//! C, and C++ callers can link against the compiled static library
//! (`libgf256_c.a`) at compile time.
//!
//! ## Why a Separate Crate?
//!
//! The `gf256` crate panics on undefined operations (division by zero, inverse
//! of zero). Panics cannot cross C frames — that is undefined behaviour. This
//! crate catches those panics and returns sentinel values instead:
//!
//! | Operation                | Error sentinel | Error flag |
//! |--------------------------|---------------|------------|
//! | `gf256_c_divide(a, 0)`   | `0xFF`        | sets error |
//! | `gf256_c_inverse(0)`     | `0xFF`        | sets error |
//!
//! Call `gf256_c_had_error()` after any operation to check whether the most
//! recent call encountered an error. The flag is reset on each new call.
//!
//! ## Thread Safety
//!
//! The error flag is stored in a thread-local variable. Each OS thread has its
//! own independent flag, which matches Swift's task/thread model.
//!
//! ## GF(256) Reminder
//!
//! GF(2^8) is the Galois Field with 256 elements (bytes 0–255). Arithmetic is:
//!
//! - **Add / Subtract**: bitwise XOR (they are the same operation in char-2).
//! - **Multiply**: log + antilog table lookup (O(1), no carry).
//! - **Divide**: log subtraction, then antilog lookup.
//! - **Power**: log scaling, then antilog lookup.
//! - **Inverse**: `a^{-1} = ALOG[255 − LOG[a]]`.
//!
//! The primitive polynomial is `x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285`.

use std::cell::Cell;
use std::panic;

// =============================================================================
// Error flag (thread-local)
// =============================================================================

// A thread-local boolean flag. `Cell<bool>` is the lightest-weight interior
// mutability primitive — no locking, no heap allocation, one byte per thread.
thread_local! {
    /// Set to `true` when the most recent C-exported function encountered
    /// an error (e.g., division by zero). Cleared at the start of each call.
    static LAST_ERROR: Cell<bool> = const { Cell::new(false) };
}

/// Mark the current thread's error flag.
#[inline]
fn set_error() {
    LAST_ERROR.with(|f| f.set(true));
}

/// Clear the current thread's error flag.
#[inline]
fn clear_error() {
    LAST_ERROR.with(|f| f.set(false));
}

// =============================================================================
// Field Operations
// =============================================================================

/// Add two GF(256) elements.
///
/// In GF(2^8), addition is bitwise XOR. No tables needed, no error cases.
///
/// ```text
/// gf256_c_add(0x53, 0xCA) == 0x99
/// gf256_c_add(x, x)       == 0  for all x
/// ```
#[no_mangle]
pub extern "C" fn gf256_c_add(a: u8, b: u8) -> u8 {
    clear_error();
    gf256::add(a, b)
}

/// Subtract two GF(256) elements.
///
/// In GF(2^8), subtraction equals addition (characteristic 2: −1 = 1).
/// This is identical to `gf256_c_add`, exposed separately for clarity.
#[no_mangle]
pub extern "C" fn gf256_c_subtract(a: u8, b: u8) -> u8 {
    clear_error();
    gf256::subtract(a, b)
}

/// Multiply two GF(256) elements using log/antilog table lookup.
///
/// Special case: `0 × anything = 0`.
///
/// ```text
/// gf256_c_multiply(2, 4)   == 8   (normal: no overflow)
/// gf256_c_multiply(2, 128) == 29  (overflow: reduced mod 0x11D)
/// gf256_c_multiply(0, 255) == 0   (zero annihilates)
/// ```
#[no_mangle]
pub extern "C" fn gf256_c_multiply(a: u8, b: u8) -> u8 {
    clear_error();
    gf256::multiply(a, b)
}

/// Divide `a` by `b` in GF(256).
///
/// `a / b = ALOG[(LOG[a] − LOG[b] + 255) % 255]`
///
/// Special case: `0 / anything = 0`.
///
/// Error case: `b = 0` — division by zero is undefined. Returns 0xFF and
/// sets the error flag. Call `gf256_c_had_error()` to detect this.
#[no_mangle]
pub extern "C" fn gf256_c_divide(a: u8, b: u8) -> u8 {
    clear_error();
    match panic::catch_unwind(|| gf256::divide(a, b)) {
        Ok(result) => result,
        Err(_) => {
            // The underlying crate panicked because b == 0.
            set_error();
            0xFF // sentinel value
        }
    }
}

/// Raise a GF(256) element to a non-negative integer power.
///
/// Uses log-table scaling: `base^exp = ALOG[(LOG[base] * exp) % 255]`.
///
/// Special cases:
/// - `0^0 = 1` (convention, matches most math libraries).
/// - `0^exp = 0` for exp > 0.
/// - `base^0 = 1` for any non-zero base.
#[no_mangle]
pub extern "C" fn gf256_c_power(base: u8, exp: u32) -> u8 {
    clear_error();
    gf256::power(base, exp)
}

/// Compute the multiplicative inverse of a GF(256) element.
///
/// Returns `a^{-1}` such that `a × a^{-1} = 1`.
///
/// Error case: `a = 0` — zero has no multiplicative inverse. Returns 0xFF
/// and sets the error flag. Call `gf256_c_had_error()` to detect this.
#[no_mangle]
pub extern "C" fn gf256_c_inverse(a: u8) -> u8 {
    clear_error();
    match panic::catch_unwind(|| gf256::inverse(a)) {
        Ok(result) => result,
        Err(_) => {
            // The underlying crate panicked because a == 0.
            set_error();
            0xFF // sentinel value
        }
    }
}

// =============================================================================
// Error Inspection
// =============================================================================

/// Return 1 if the most recent call on this thread encountered an error.
///
/// The flag is reset at the beginning of every `gf256_c_*` call, so this
/// must be checked *immediately* after the call whose result is in question.
///
/// ```c
/// uint8_t result = gf256_c_divide(42, 0);
/// if (gf256_c_had_error()) {
///     // handle division-by-zero error
/// }
/// ```
#[no_mangle]
pub extern "C" fn gf256_c_had_error() -> u8 {
    LAST_ERROR.with(|f| if f.get() { 1 } else { 0 })
}

// =============================================================================
// Constants
// =============================================================================

/// Return the primitive polynomial used for GF(256).
///
/// This is `x^8 + x^4 + x^3 + x^2 + 1`, represented as the integer 285
/// (binary: 0b1_0001_1101, hex: 0x11D).
///
/// Useful for callers that need to document or replicate the field definition.
#[no_mangle]
pub extern "C" fn gf256_c_primitive_polynomial() -> u32 {
    gf256::PRIMITIVE_POLYNOMIAL as u32
}
