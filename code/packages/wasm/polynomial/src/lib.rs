//! # polynomial-wasm
//!
//! WebAssembly build of the [`polynomial`] crate, exposing all polynomial
//! arithmetic operations through a plain C ABI (no wasm-bindgen, no JS glue).
//!
//! ## What Is WebAssembly?
//!
//! WebAssembly (WASM) is a compact binary instruction format designed to run at
//! near-native speed inside a sandbox. It has four value types:
//!
//! ```text
//! i32, i64   — 32- and 64-bit integers
//! f32, f64   — 32- and 64-bit floats
//! ```
//!
//! WASM modules have a single flat **linear memory** — a byte array shared between
//! the module and its host (the WASM runtime). Functions can only exchange those
//! four scalar types; they cannot return structs or slices directly.
//!
//! ## The Memory Protocol
//!
//! Because polynomial operations take and return variable-length arrays, we need a
//! convention for passing slices between the host and this module:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │  Host (WASM runtime)          │  polynomial-wasm module            │
//! ├────────────────────────────────────────────────────────────────────┤
//! │  1. ptr = poly_alloc(n)       │  allocates n × 8 bytes in          │
//! │                               │  linear memory, returns pointer     │
//! │  2. mem[ptr..] = [a0,a1,…]   │  host writes f64 coefficients       │
//! │  3. res = poly_add(…)         │  returns pointer to result array    │
//! │  4. len = poly_last_result_len│  how many f64s in the result        │
//! │  5. read mem[res..res+len*8]  │  host reads result coefficients     │
//! │  6. poly_dealloc(res, len)    │  host frees the result memory       │
//! └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! **Why allocate inside the module?**
//! The WASM module manages its own heap. The host cannot safely `malloc` inside
//! the module's address space; it can only write to memory the module has
//! already allocated and handed back as a pointer.
//!
//! **Ownership rule:** Every pointer returned by a function in this module is
//! owned by the caller. The caller MUST eventually call `poly_dealloc(ptr, len)`
//! to avoid a memory leak. (WASM linear memory does not shrink automatically.)
//!
//! ## Error Handling
//!
//! WASM with `panic = "abort"` traps on panic, which terminates the module. To
//! avoid this for invalid inputs (e.g., division by zero), we wrap calls with
//! `std::panic::catch_unwind`. If a panic is caught, `poly_had_error()` returns 1
//! and the function returns a null pointer or 0. Always check `poly_had_error()`
//! after any operation that could panic.
//!
//! ## Divmod Protocol
//!
//! Polynomial division returns two values (quotient and remainder) but a C
//! function can return only one. We cache both results in module-level statics and
//! expose four accessor functions:
//!
//! ```text
//! poly_divmod(…)                → computes and caches both results
//! poly_divmod_quotient_ptr()    → pointer to cached quotient array
//! poly_divmod_quotient_len()    → length of cached quotient array
//! poly_divmod_remainder_ptr()   → pointer to cached remainder array
//! poly_divmod_remainder_len()   → length of cached remainder array
//! ```
//!
//! The host is responsible for calling `poly_dealloc` on the quotient and
//! remainder pointers after reading them. The next call to `poly_divmod` will
//! free the old cached pointers automatically before overwriting (Fix 4).

use std::alloc::{alloc, dealloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

// =============================================================================
// Error flag
// =============================================================================

/// Set to `true` when a call panicked (e.g., division by zero).
///
/// We use an `AtomicBool` rather than `static mut bool` to eliminate the
/// undefined behaviour that `static mut` causes when read and written from
/// multiple contexts (even though WASM is single-threaded today, the compiler
/// does not know that and may misoptimise around `static mut` accesses).
/// Using `Ordering::Relaxed` is safe here: WASM is single-threaded so there
/// are no concurrent stores from another thread that could race with our load.
static LAST_ERROR: AtomicBool = AtomicBool::new(false);

/// Returns 1 if the most recent operation set the error flag, 0 otherwise.
///
/// The error flag is cleared at the beginning of each operation. Check this
/// after any call that could panic (divmod, divide, modulo, gcd) to detect
/// invalid inputs such as division by zero.
///
/// ## Example (host pseudocode)
///
/// ```text
/// poly_divmod(div_ptr, div_len, sor_ptr, sor_len);
/// if poly_had_error() != 0 {
///     // divisor was zero; result pointers are null
/// }
/// ```
#[no_mangle]
pub extern "C" fn poly_had_error() -> u32 {
    LAST_ERROR.load(Ordering::Relaxed) as u32
}

// =============================================================================
// Memory allocation
// =============================================================================

/// Allocate `len` consecutive `f64` values in WASM linear memory.
///
/// Returns a pointer to the first `f64`. The host can write coefficients into
/// this region immediately after this call.
///
/// For `len == 0`, returns a non-null dangling pointer. Callers MUST NOT
/// dereference this pointer; it is only a sentinel to distinguish "valid empty
/// allocation" from "allocation failed" (null).
///
/// ## Why 8-byte alignment?
///
/// An `f64` is 8 bytes. The Rust allocator returns memory aligned to 8 bytes
/// for `f64` arrays, which is required for correct WASM memory access.
///
/// ## On allocation failure
///
/// Sets the error flag and returns null. The caller should check
/// `poly_had_error()` if a null is returned for a non-zero `len`.
#[no_mangle]
pub unsafe extern "C" fn poly_alloc(len: u32) -> *mut f64 {
    if len == 0 {
        // Return a non-null dangling pointer for zero-length allocations.
        // Callers must not dereference this pointer.
        return std::ptr::NonNull::<f64>::dangling().as_ptr();
    }
    let layout = match Layout::array::<f64>(len as usize) {
        Ok(l) => l,
        Err(_) => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let ptr = alloc(layout) as *mut f64;
    if ptr.is_null() {
        LAST_ERROR.store(true, Ordering::Relaxed);
    }
    ptr
}

/// Deallocate a slice of `len` `f64` values previously returned by this module.
///
/// The host MUST call this for every non-empty pointer returned by a polynomial
/// operation, otherwise the module's heap will leak and eventually exhaust WASM
/// linear memory.
///
/// For `len == 0` or a null `ptr`, this is a no-op (safe to call).
///
/// ## Safety
///
/// - `ptr` must have been returned by `poly_alloc(len)` or by a polynomial
///   operation (which calls `poly_alloc` internally).
/// - `len` must match the length used when the pointer was allocated.
/// - Must not be called twice on the same pointer (double-free).
#[no_mangle]
pub unsafe extern "C" fn poly_dealloc(ptr: *mut f64, len: u32) {
    if len == 0 {
        return; // dangling pointer from a zero-length alloc — nothing to free
    }
    if ptr.is_null() {
        return;
    }
    let layout = match Layout::array::<f64>(len as usize) {
        Ok(l) => l,
        Err(_) => return,
    };
    dealloc(ptr as *mut u8, layout);
}

// =============================================================================
// Result length
// =============================================================================

/// The length (number of f64 elements) of the most recent result array.
///
/// After any function that returns `*mut f64`, call this to learn how many
/// coefficients are in the result before reading them.
///
/// Using `AtomicU32` instead of `static mut u32` avoids undefined behaviour
/// from mutable statics (see comment on `LAST_ERROR` above).
static LAST_RESULT_LEN: AtomicU32 = AtomicU32::new(0);

/// Returns the number of `f64` elements in the most recently returned array.
///
/// This value is set by every function that returns a `*mut f64`. It reflects
/// the coefficient count of the polynomial stored at that pointer.
///
/// ## Usage pattern
///
/// ```text
/// let ptr = poly_add(a_ptr, a_len, b_ptr, b_len);
/// let len = poly_last_result_len();
/// // read len f64s from ptr
/// poly_dealloc(ptr, len);
/// ```
#[no_mangle]
pub extern "C" fn poly_last_result_len() -> u32 {
    LAST_RESULT_LEN.load(Ordering::Relaxed)
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Copy a `Vec<f64>` into WASM-allocated memory, set `LAST_RESULT_LEN`, and
/// return the pointer.
///
/// If `v` is empty (the zero polynomial), returns null and sets len=0.
/// The caller (host) should treat a null pointer as the zero polynomial.
unsafe fn vec_to_wasm(v: Vec<f64>) -> *mut f64 {
    // Fix 3: guard against vectors so large the length would overflow u32.
    // In practice this cannot happen in WASM (4 GiB address space), but the
    // check makes the cast provably safe and the error path explicit.
    if v.len() > u32::MAX as usize {
        LAST_ERROR.store(true, Ordering::Relaxed);
        LAST_RESULT_LEN.store(0, Ordering::Relaxed);
        return std::ptr::null_mut();
    }
    LAST_RESULT_LEN.store(v.len() as u32, Ordering::Relaxed);
    if v.is_empty() {
        return std::ptr::null_mut();
    }
    // Allocate module-owned memory for the result.
    let ptr = poly_alloc(v.len() as u32);
    if ptr.is_null() {
        // poly_alloc already set LAST_ERROR; leave LAST_RESULT_LEN as set above.
        return std::ptr::null_mut();
    }
    // Copy coefficients from the temporary Vec into the allocated region.
    std::ptr::copy_nonoverlapping(v.as_ptr(), ptr, v.len());
    // `v` is dropped here; its heap storage is freed. The copy in `ptr` persists.
    ptr
}

/// Convert a raw pointer + length into a shared slice, with null and len==0 safety.
///
/// - If `len == 0`, returns an empty slice regardless of `ptr` (even if null).
///   This is correct because `slice::from_raw_parts` with len=0 requires a
///   non-null, aligned pointer, but empty polynomials are represented as null
///   by this module's convention.
/// - If `ptr` is null and `len > 0`, the caller passed an invalid pointer for a
///   non-empty polynomial. Returns `None` so the caller can set the error flag
///   and return early.
/// - Otherwise delegates to `slice::from_raw_parts`, which is sound provided
///   the caller supplied a valid, aligned, live pointer (as documented on every
///   exported function).
///
/// # Safety
///
/// The caller must ensure that when `ptr` is non-null and `len > 0`, `ptr`
/// points to at least `len` initialised, aligned `f64` values that remain live
/// for the lifetime `'a`.
unsafe fn as_slice<'a>(ptr: *const f64, len: u32) -> Option<&'a [f64]> {
    if len == 0 {
        return Some(&[]);
    }
    if ptr.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts(ptr, len as usize))
}

// =============================================================================
// Divmod cache
// =============================================================================

/// Cached quotient pointer from the most recent `poly_divmod` call.
///
/// Stored as an `AtomicPtr<f64>` to avoid the undefined behaviour of `static mut`.
/// WASM is single-threaded, so `Ordering::Relaxed` loads and stores are correct.
static DIVMOD_QUOTIENT_PTR: AtomicPtr<f64> = AtomicPtr::new(std::ptr::null_mut());
/// Cached quotient length from the most recent `poly_divmod` call.
static DIVMOD_QUOTIENT_LEN: AtomicU32 = AtomicU32::new(0);
/// Cached remainder pointer from the most recent `poly_divmod` call.
static DIVMOD_REMAINDER_PTR: AtomicPtr<f64> = AtomicPtr::new(std::ptr::null_mut());
/// Cached remainder length from the most recent `poly_divmod` call.
static DIVMOD_REMAINDER_LEN: AtomicU32 = AtomicU32::new(0);

// =============================================================================
// Exported polynomial operations
// =============================================================================

/// Normalize a polynomial: strip trailing near-zero coefficients.
///
/// `[1.0, 0.0, 0.0]` and `[1.0]` represent the same polynomial; after
/// normalization both become `[1.0]`.
///
/// ## Parameters
/// - `ptr` — pointer to the coefficient array (lowest degree first)
/// - `len` — number of coefficients
///
/// ## Returns
/// Pointer to the normalized result. Use `poly_last_result_len()` for the length.
/// Caller must free with `poly_dealloc`. Returns null and sets error on null input
/// with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_normalize(ptr: *const f64, len: u32) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(ptr, len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let result = polynomial::normalize(a);
    vec_to_wasm(result)
}

/// Return the degree of a polynomial.
///
/// The degree is the index of the highest non-zero coefficient. For the zero
/// polynomial `[]`, returns 0 (by the convention in the polynomial crate).
///
/// ## Parameters
/// - `ptr` — pointer to the coefficient array
/// - `len` — number of coefficients
///
/// ## Returns
/// Degree as a `u32`. No heap allocation; no need to call `poly_dealloc`.
/// Returns 0 and sets error flag if ptr is null with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_degree(ptr: *const f64, len: u32) -> u32 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(ptr, len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            return 0;
        }
    };
    polynomial::degree(a) as u32
}

/// Add two polynomials: `a + b`.
///
/// Coefficients are added position-by-position. If one polynomial is shorter,
/// its missing high-degree terms are treated as zero.
///
/// ## Parameters
/// - `a_ptr`, `a_len` — first polynomial
/// - `b_ptr`, `b_len` — second polynomial
///
/// ## Returns
/// Pointer to result. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error if either pointer is null with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_add(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let result = polynomial::add(a, b);
    vec_to_wasm(result)
}

/// Subtract polynomial `b` from `a`: `a - b`.
///
/// In ordinary real-coefficient polynomial arithmetic, subtraction differs from
/// addition. (Contrast with GF(2^8) where they are identical.)
///
/// ## Parameters
/// - `a_ptr`, `a_len` — minuend
/// - `b_ptr`, `b_len` — subtrahend
///
/// ## Returns
/// Pointer to result. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error if either pointer is null with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_subtract(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let result = polynomial::subtract(a, b);
    vec_to_wasm(result)
}

/// Multiply two polynomials: `a × b`.
///
/// Uses polynomial convolution: each term `a[i]·xⁱ` multiplies each term
/// `b[j]·xʲ`, contributing `a[i]·b[j]` to the result at index `i+j`.
///
/// The result has degree `deg(a) + deg(b)`.
///
/// ## Parameters
/// - `a_ptr`, `a_len` — first polynomial
/// - `b_ptr`, `b_len` — second polynomial
///
/// ## Returns
/// Pointer to result. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error if either pointer is null with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_multiply(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let result = polynomial::multiply(a, b);
    vec_to_wasm(result)
}

/// Perform polynomial long division, caching both quotient and remainder.
///
/// After this call, retrieve results with:
/// - `poly_divmod_quotient_ptr()` / `poly_divmod_quotient_len()`
/// - `poly_divmod_remainder_ptr()` / `poly_divmod_remainder_len()`
///
/// **Free both** with `poly_dealloc` after reading. The next call to
/// `poly_divmod` will automatically free the previously cached pointers before
/// overwriting them, so not freeing between calls does not leak memory.
///
/// ## Parameters
/// - `dividend_ptr`, `dividend_len` — the numerator polynomial
/// - `divisor_ptr`, `divisor_len` — the denominator polynomial (must be non-zero)
///
/// ## On error (divisor is zero polynomial, or null pointer with non-zero len)
///
/// Sets `poly_had_error() = 1`. Both cached pointers are set to null, lengths to 0.
#[no_mangle]
pub unsafe extern "C" fn poly_divmod(
    dividend_ptr: *const f64,
    dividend_len: u32,
    divisor_ptr: *const f64,
    divisor_len: u32,
) {
    LAST_ERROR.store(false, Ordering::Relaxed);

    // Free previous divmod results before overwriting (Fix 4: prevent memory leak).
    let old_q_ptr = DIVMOD_QUOTIENT_PTR.load(Ordering::Relaxed);
    let old_q_len = DIVMOD_QUOTIENT_LEN.load(Ordering::Relaxed);
    if !old_q_ptr.is_null() && old_q_len > 0 {
        poly_dealloc(old_q_ptr, old_q_len);
    }
    let old_r_ptr = DIVMOD_REMAINDER_PTR.load(Ordering::Relaxed);
    let old_r_len = DIVMOD_REMAINDER_LEN.load(Ordering::Relaxed);
    if !old_r_ptr.is_null() && old_r_len > 0 {
        poly_dealloc(old_r_ptr, old_r_len);
    }
    DIVMOD_QUOTIENT_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
    DIVMOD_REMAINDER_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
    DIVMOD_QUOTIENT_LEN.store(0, Ordering::Relaxed);
    DIVMOD_REMAINDER_LEN.store(0, Ordering::Relaxed);

    let dividend = match as_slice(dividend_ptr, dividend_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            return;
        }
    };
    let divisor = match as_slice(divisor_ptr, divisor_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            return;
        }
    };

    // Wrap the call in catch_unwind so a "division by zero" panic doesn't trap.
    let result = std::panic::catch_unwind(|| polynomial::divmod(dividend, divisor));

    match result {
        Ok((quot, rem)) => {
            // Copy quotient into WASM-owned memory and cache the pointer+length.
            DIVMOD_QUOTIENT_LEN.store(quot.len() as u32, Ordering::Relaxed);
            if quot.is_empty() {
                DIVMOD_QUOTIENT_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
            } else {
                let q_ptr = poly_alloc(quot.len() as u32);
                // Fix 2: check for allocation failure before dereferencing.
                if q_ptr.is_null() {
                    LAST_ERROR.store(true, Ordering::Relaxed);
                    DIVMOD_QUOTIENT_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
                    DIVMOD_QUOTIENT_LEN.store(0, Ordering::Relaxed);
                    DIVMOD_REMAINDER_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
                    DIVMOD_REMAINDER_LEN.store(0, Ordering::Relaxed);
                    return;
                }
                std::ptr::copy_nonoverlapping(quot.as_ptr(), q_ptr, quot.len());
                DIVMOD_QUOTIENT_PTR.store(q_ptr, Ordering::Relaxed);
            }

            // Copy remainder into WASM-owned memory and cache.
            DIVMOD_REMAINDER_LEN.store(rem.len() as u32, Ordering::Relaxed);
            if rem.is_empty() {
                DIVMOD_REMAINDER_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
            } else {
                let r_ptr = poly_alloc(rem.len() as u32);
                // Fix 2: check for allocation failure before dereferencing.
                if r_ptr.is_null() {
                    LAST_ERROR.store(true, Ordering::Relaxed);
                    // quotient was already stored successfully; free it to avoid a leak.
                    let q_ptr = DIVMOD_QUOTIENT_PTR.load(Ordering::Relaxed);
                    let q_len = DIVMOD_QUOTIENT_LEN.load(Ordering::Relaxed);
                    if !q_ptr.is_null() && q_len > 0 {
                        poly_dealloc(q_ptr, q_len);
                    }
                    DIVMOD_QUOTIENT_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
                    DIVMOD_QUOTIENT_LEN.store(0, Ordering::Relaxed);
                    DIVMOD_REMAINDER_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
                    DIVMOD_REMAINDER_LEN.store(0, Ordering::Relaxed);
                    return;
                }
                std::ptr::copy_nonoverlapping(rem.as_ptr(), r_ptr, rem.len());
                DIVMOD_REMAINDER_PTR.store(r_ptr, Ordering::Relaxed);
            }
        }
        Err(_) => {
            // The polynomial crate panicked (divisor was the zero polynomial).
            LAST_ERROR.store(true, Ordering::Relaxed);
            DIVMOD_QUOTIENT_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
            DIVMOD_QUOTIENT_LEN.store(0, Ordering::Relaxed);
            DIVMOD_REMAINDER_PTR.store(std::ptr::null_mut(), Ordering::Relaxed);
            DIVMOD_REMAINDER_LEN.store(0, Ordering::Relaxed);
        }
    }
}

/// Return the cached quotient pointer from the most recent `poly_divmod` call.
///
/// Null if the divisor was zero or if the quotient is the zero polynomial.
/// The host owns this memory and must call `poly_dealloc(ptr, len)` after use.
/// Stored in an `AtomicPtr<f64>`; this accessor uses `Ordering::Relaxed`.
#[no_mangle]
pub extern "C" fn poly_divmod_quotient_ptr() -> *mut f64 {
    DIVMOD_QUOTIENT_PTR.load(Ordering::Relaxed)
}

/// Return the number of f64 elements in the cached quotient.
#[no_mangle]
pub extern "C" fn poly_divmod_quotient_len() -> u32 {
    DIVMOD_QUOTIENT_LEN.load(Ordering::Relaxed)
}

/// Return the cached remainder pointer from the most recent `poly_divmod` call.
///
/// Null if the divisor was zero or if the remainder is the zero polynomial.
/// The host owns this memory and must call `poly_dealloc(ptr, len)` after use.
/// Stored in an `AtomicPtr<f64>`; this accessor uses `Ordering::Relaxed`.
#[no_mangle]
pub extern "C" fn poly_divmod_remainder_ptr() -> *mut f64 {
    DIVMOD_REMAINDER_PTR.load(Ordering::Relaxed)
}

/// Return the number of f64 elements in the cached remainder.
#[no_mangle]
pub extern "C" fn poly_divmod_remainder_len() -> u32 {
    DIVMOD_REMAINDER_LEN.load(Ordering::Relaxed)
}

/// Return the quotient of `a / b`.
///
/// Internally calls `polynomial::divide`, which panics on a zero divisor.
/// On panic, sets the error flag and returns null.
///
/// ## Returns
/// Pointer to quotient. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error on null input pointer with non-zero len, or on panic.
#[no_mangle]
pub unsafe extern "C" fn poly_divide(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    match std::panic::catch_unwind(|| polynomial::divide(a, b)) {
        Ok(result) => vec_to_wasm(result),
        Err(_) => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            std::ptr::null_mut()
        }
    }
}

/// Return the remainder of `a / b` (i.e., `a mod b`).
///
/// Used in GF(2^8) construction to reduce high-degree polynomials modulo the
/// primitive polynomial. On zero divisor panic, sets error flag and returns null.
///
/// ## Returns
/// Pointer to remainder. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error on null input pointer with non-zero len, or on panic.
#[no_mangle]
pub unsafe extern "C" fn poly_modulo(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    match std::panic::catch_unwind(|| polynomial::modulo(a, b)) {
        Ok(result) => vec_to_wasm(result),
        Err(_) => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            std::ptr::null_mut()
        }
    }
}

/// Evaluate a polynomial at a single point `x` using Horner's method.
///
/// Horner's method computes `a₀ + x(a₁ + x(a₂ + … + x·aₙ))` in O(n) time
/// without any exponentiation. This is the most numerically stable form.
///
/// ## Parameters
/// - `ptr`, `len` — the polynomial coefficients (lowest degree first)
/// - `x` — the point at which to evaluate
///
/// ## Returns
/// The scalar result as an `f64`. No heap allocation; no `poly_dealloc` needed.
/// Returns 0.0 and sets error flag if ptr is null with non-zero len.
#[no_mangle]
pub unsafe extern "C" fn poly_evaluate(ptr: *const f64, len: u32, x: f64) -> f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(ptr, len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            return 0.0;
        }
    };
    polynomial::evaluate(a, x)
}

/// Compute the greatest common divisor of two polynomials using the Euclidean algorithm.
///
/// The GCD is the highest-degree polynomial dividing both inputs with zero remainder.
/// Uses repeated polynomial `modulo` until the remainder is the zero polynomial,
/// analogous to the integer Euclidean algorithm.
///
/// On division-by-zero panic (e.g., both inputs are zero), sets error flag and
/// returns null.
///
/// ## Returns
/// Pointer to GCD polynomial. Length via `poly_last_result_len()`. Free with `poly_dealloc`.
/// Returns null and sets error on null input pointer with non-zero len, or on panic.
#[no_mangle]
pub unsafe extern "C" fn poly_gcd(
    a_ptr: *const f64,
    a_len: u32,
    b_ptr: *const f64,
    b_len: u32,
) -> *mut f64 {
    LAST_ERROR.store(false, Ordering::Relaxed);
    let a = match as_slice(a_ptr, a_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    let b = match as_slice(b_ptr, b_len) {
        Some(s) => s,
        None => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            return std::ptr::null_mut();
        }
    };
    match std::panic::catch_unwind(|| polynomial::gcd(a, b)) {
        Ok(result) => vec_to_wasm(result),
        Err(_) => {
            LAST_ERROR.store(true, Ordering::Relaxed);
            LAST_RESULT_LEN.store(0, Ordering::Relaxed);
            std::ptr::null_mut()
        }
    }
}
