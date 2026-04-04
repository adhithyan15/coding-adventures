//! # gf256-wasm
//!
//! WebAssembly build of the [`gf256`] crate, exposing GF(2^8) arithmetic
//! through a plain C ABI (no wasm-bindgen, no JS glue).
//!
//! ## GF(2^8) in 30 seconds
//!
//! **GF(2^8)** (read "GF of 256") is a finite field with exactly 256 elements
//! — the integers 0 through 255. The arithmetic works like this:
//!
//! - **Addition** = XOR (characteristic-2 field: `1 + 1 = 0`)
//! - **Subtraction** = XOR (same as addition, because `-1 = 1`)
//! - **Multiplication** = polynomial multiplication modulo `0x11D`
//! - **Division / inverse** = computed via logarithm tables
//!
//! GF(256) powers Reed-Solomon error correction (QR codes, CDs, hard drives)
//! and is the field underlying AES encryption.
//!
//! ## WASM Value Types
//!
//! WebAssembly has no native byte type. The smallest integer type is `i32`
//! (32-bit). All GF(256) values — which fit in one byte (0–255) — are passed
//! as `u32`. The caller is responsible for ensuring values are in `[0, 255]`.
//! The module reads only the low 8 bits via `as u8`.
//!
//! ## No Memory Protocol Needed
//!
//! Unlike the polynomial WASM module, GF(256) operations are purely scalar:
//! one or two byte inputs, one byte output. WASM can pass these directly as
//! `i32` values. No heap allocation, no `alloc`/`dealloc`, no pointers.
//!
//! ## Error Handling
//!
//! Two operations can fail:
//!
//! - **`gf256_divide(a, 0)`** — division by zero is undefined in any field.
//! - **`gf256_inverse(0)`** — zero has no multiplicative inverse.
//!
//! In both cases, rather than trapping (which would terminate the WASM module),
//! we return `0xFF` (255, an otherwise valid field element that won't be
//! confused with a legitimate result in most contexts) and set the error flag.
//! Always check `gf256_had_error()` after divide or inverse operations.
//!
//! The error flag is cleared at the start of each operation, so it reflects
//! only the most recent call.

// =============================================================================
// Error flag
// =============================================================================

/// Set to `true` when a call panicked (division by zero, inverse of zero).
///
/// WASM is single-threaded, so a mutable static is safe here.
static mut LAST_ERROR: bool = false;

/// Returns 1 if the most recent operation set the error flag, 0 otherwise.
///
/// The error flag is cleared at the beginning of each operation call. After
/// calling `gf256_divide` or `gf256_inverse`, check this to detect invalid
/// inputs before using the return value.
///
/// ## Example (host pseudocode)
///
/// ```text
/// let result = gf256_inverse(0);
/// if gf256_had_error() != 0 {
///     // error: zero has no inverse; result is 0xFF (sentinel)
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn gf256_had_error() -> u32 {
    LAST_ERROR as u32
}

// =============================================================================
// Arithmetic operations
// =============================================================================

/// Add two GF(256) elements.
///
/// In a characteristic-2 field, addition is bitwise XOR. No tables needed.
///
/// ```text
/// gf256_add(0x53, 0xCA) = 0x53 ^ 0xCA = 0x99
/// gf256_add(x, x)       = 0  for all x   (every element is its own inverse)
/// ```
///
/// ## Parameters
/// - `a` — first element (u32; low 8 bits used, must be 0–255)
/// - `b` — second element (u32; low 8 bits used, must be 0–255)
///
/// ## Returns
/// `a XOR b` as a u32 (high bits are 0).
#[no_mangle]
pub unsafe extern "C" fn gf256_add(a: u32, b: u32) -> u32 {
    LAST_ERROR = false;
    // Truncate to u8 before passing to the gf256 crate.
    gf256::add(a as u8, b as u8) as u32
}

/// Subtract two GF(256) elements.
///
/// In GF(2^8), subtraction is identical to addition (both are XOR) because
/// the field has characteristic 2: every element is its own additive inverse.
///
/// ```text
/// gf256_subtract(a, b) = a XOR b   (same as gf256_add)
/// ```
///
/// ## Parameters
/// - `a` — minuend (u32; low 8 bits used, must be 0–255)
/// - `b` — subtrahend (u32; low 8 bits used, must be 0–255)
///
/// ## Returns
/// `a XOR b` as a u32 (high bits are 0).
#[no_mangle]
pub unsafe extern "C" fn gf256_subtract(a: u32, b: u32) -> u32 {
    LAST_ERROR = false;
    gf256::subtract(a as u8, b as u8) as u32
}

/// Multiply two GF(256) elements using logarithm/antilogarithm tables.
///
/// Uses the identity: `a × b = ALOG[(LOG[a] + LOG[b]) % 255]`.
/// The tables are precomputed at first use via `std::sync::OnceLock`.
///
/// ## Special case
///
/// `0 × anything = 0` (zero is the additive identity; it is not reachable as
/// a power of the generator, so the log-table path is bypassed).
///
/// ## Parameters
/// - `a` — first element (u32; low 8 bits used, must be 0–255)
/// - `b` — second element (u32; low 8 bits used, must be 0–255)
///
/// ## Returns
/// Product in GF(256) as u32 (0–255).
#[no_mangle]
pub unsafe extern "C" fn gf256_multiply(a: u32, b: u32) -> u32 {
    LAST_ERROR = false;
    gf256::multiply(a as u8, b as u8) as u32
}

/// Divide `a` by `b` in GF(256).
///
/// `a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255]`
///
/// The `+ 255` prevents underflow when `LOG[a] < LOG[b]`.
///
/// ## Special case
///
/// `0 / b = 0` for any non-zero `b`.
///
/// ## On error (b == 0)
///
/// Division by zero is undefined in any field. Returns `0xFF` (255) and sets
/// `gf256_had_error() = 1`.
///
/// ## Parameters
/// - `a` — dividend (u32; low 8 bits used, must be 0–255)
/// - `b` — divisor (u32; low 8 bits used, must be 0–255; must not be 0)
///
/// ## Returns
/// Quotient in GF(256) as u32 (0–255), or 0xFF on error.
#[no_mangle]
pub unsafe extern "C" fn gf256_divide(a: u32, b: u32) -> u32 {
    LAST_ERROR = false;
    match std::panic::catch_unwind(|| gf256::divide(a as u8, b as u8)) {
        Ok(result) => result as u32,
        Err(_) => {
            LAST_ERROR = true;
            // Return 0xFF as a sentinel. Callers should check gf256_had_error().
            0xFF
        }
    }
}

/// Raise a GF(256) element to a non-negative integer power.
///
/// `base^exp = ALOG[(LOG[base] × exp) % 255]`
///
/// The modulo 255 reflects the multiplicative group order: every non-zero
/// element satisfies `g^255 = 1` (Fermat's little theorem for finite fields).
///
/// ## Special cases
///
/// - `0^0 = 1` (by mathematical convention)
/// - `0^exp = 0` for exp > 0
/// - `base^0 = 1` for any non-zero base
///
/// ## Parameters
/// - `base` — base element (u32; low 8 bits used, must be 0–255)
/// - `exp` — exponent (full u32 range; large exponents are fine because
///   `(LOG[base] × exp) % 255` is computed in u64 to avoid overflow)
///
/// ## Returns
/// `base^exp` in GF(256) as u32 (0–255).
#[no_mangle]
pub unsafe extern "C" fn gf256_power(base: u32, exp: u32) -> u32 {
    LAST_ERROR = false;
    gf256::power(base as u8, exp) as u32
}

/// Compute the multiplicative inverse of a GF(256) element.
///
/// The inverse of `a` satisfies: `a × inverse(a) = 1`.
///
/// Derived from the log table: `a^(-1) = ALOG[255 - LOG[a]]`.
///
/// ## On error (a == 0)
///
/// Zero has no multiplicative inverse — no field element times zero equals 1.
/// Returns `0xFF` (255) and sets `gf256_had_error() = 1`.
///
/// ## Parameters
/// - `a` — the element to invert (u32; low 8 bits used, must be 1–255)
///
/// ## Returns
/// `a^(-1)` in GF(256) as u32 (0–255), or 0xFF on error.
#[no_mangle]
pub unsafe extern "C" fn gf256_inverse(a: u32) -> u32 {
    LAST_ERROR = false;
    match std::panic::catch_unwind(|| gf256::inverse(a as u8)) {
        Ok(result) => result as u32,
        Err(_) => {
            LAST_ERROR = true;
            // Return 0xFF as a sentinel. Callers should check gf256_had_error().
            0xFF
        }
    }
}

// =============================================================================
// Constants
// =============================================================================

/// Return the additive identity element of GF(256): 0.
///
/// For any element `a`: `gf256_add(a, gf256_zero()) = a`.
///
/// Provided as a function for symmetry with `gf256_one()` and to make WASM
/// host code more self-documenting.
#[no_mangle]
pub unsafe extern "C" fn gf256_zero() -> u32 {
    gf256::ZERO as u32
}

/// Return the multiplicative identity element of GF(256): 1.
///
/// For any element `a`: `gf256_multiply(a, gf256_one()) = a`.
#[no_mangle]
pub unsafe extern "C" fn gf256_one() -> u32 {
    gf256::ONE as u32
}

/// Return the primitive (irreducible) polynomial used for modular reduction.
///
/// ```text
/// p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
/// ```
///
/// This polynomial is both irreducible (cannot be factored over GF(2)) and
/// primitive (the element `g = 2` generates all 255 non-zero elements of GF(256)).
///
/// Returned as a u32 because the value 285 does not fit in a u8.
///
/// Note: AES uses a different primitive polynomial (`0x11B`); this crate
/// uses `0x11D` (the same one used in Reed-Solomon implementations).
#[no_mangle]
pub unsafe extern "C" fn gf256_primitive_polynomial() -> u32 {
    gf256::PRIMITIVE_POLYNOMIAL as u32
}
