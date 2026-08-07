//! # caesar-cipher-native — a C ABI over the Rust `caesar-cipher` crate
//!
//! This crate is the *native-through-Rust* half of the Dart `caesar-cipher`
//! package. The pure-Dart port reimplements the algorithm; this one **reuses
//! the exact Rust implementation** and exposes it across a stable C ABI so
//! that Dart (via `dart:ffi`) — or any C-FFI-capable language — can call it.
//!
//! ## Why a C ABI and not a Dart-specific bridge?
//!
//! The Caesar cipher is a set of *pure functions*: `String -> String`. There
//! is no threading, no callbacks, no shared mutable state. The simplest and
//! most portable contract is therefore a handful of `extern "C"` functions
//! that take and return C strings, exactly like the repo's `c-bridge` pattern.
//! (Contrast `conduit-dart-bridge`, which needs a thread-safe post+block
//! channel *because* it delivers callbacks from a Rust background thread.)
//!
//! ## The memory contract
//!
//! Every function that returns a `*mut c_char` transfers ownership of a
//! heap-allocated, NUL-terminated UTF-8 string to the caller. The caller
//! **must** hand that pointer back to [`caesar_free_string`] exactly once.
//! Failing to do so leaks; freeing twice or freeing a pointer we did not
//! allocate is undefined behaviour — the standard C-ABI string contract.
//!
//! ```text
//!   Dart                         Rust (this crate)
//!   ----                         -----------------
//!   text.toNativeUtf8()  ─────▶  caesar_encrypt(ptr, shift)
//!                                    → CString::into_raw()  (Rust owns nothing now)
//!   result.toDartString() ◀────  *mut c_char
//!   caesar_free_string(result) ▶  CString::from_raw() drops it
//! ```
//!
//! ## NUL bytes
//!
//! A C string cannot contain an interior NUL byte. The reference cipher passes
//! arbitrary non-letter bytes through unchanged, but text carrying a literal
//! `\0` cannot cross a C-string boundary; such input is treated as empty. This
//! is a property of the C ABI, not of the cipher, and is the one behavioural
//! caveat relative to the in-process Rust/Dart APIs.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use caesar_cipher::{analysis, cipher};

// ===================================================================
// Small helpers for crossing the C boundary
// ===================================================================

/// Borrow a C string as a Rust `&str`. Returns `""` for a null pointer or
/// non-UTF-8 bytes, so callers never see a panic across the FFI boundary.
///
/// # Safety
/// `ptr` must be either null or a valid, NUL-terminated C string that stays
/// alive for the duration of the call.
unsafe fn borrow_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

/// Move a Rust `String` onto the C heap, transferring ownership to the caller.
/// If the string somehow contains an interior NUL, we fall back to an empty
/// C string rather than panicking.
fn into_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::new("").unwrap().into_raw(),
    }
}

// ===================================================================
// Public C ABI
// ===================================================================

/// Encrypt `text` with the Caesar cipher using `shift`.
/// Returns a newly allocated C string; free it with [`caesar_free_string`].
///
/// # Safety
/// `text` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn caesar_encrypt(text: *const c_char, shift: c_int) -> *mut c_char {
    into_c_string(cipher::encrypt(borrow_str(text), shift))
}

/// Decrypt `text` (inverse of [`caesar_encrypt`]) using `shift`.
/// Returns a newly allocated C string; free it with [`caesar_free_string`].
///
/// # Safety
/// `text` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn caesar_decrypt(text: *const c_char, shift: c_int) -> *mut c_char {
    into_c_string(cipher::decrypt(borrow_str(text), shift))
}

/// Apply ROT13 to `text` (shift 13, self-inverse).
/// Returns a newly allocated C string; free it with [`caesar_free_string`].
///
/// # Safety
/// `text` must be null or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn caesar_rot13(text: *const c_char) -> *mut c_char {
    into_c_string(cipher::rot13(borrow_str(text)))
}

/// Recover the most likely shift for `ciphertext` via chi-squared frequency
/// analysis. The best shift is written to `out_shift` (if non-null) and the
/// corresponding plaintext is returned as a newly allocated C string; free it
/// with [`caesar_free_string`].
///
/// # Safety
/// `ciphertext` must be null or a valid NUL-terminated C string. `out_shift`
/// must be null or a valid, writable `*mut c_int`.
#[no_mangle]
pub unsafe extern "C" fn caesar_frequency_analysis(
    ciphertext: *const c_char,
    out_shift: *mut c_int,
) -> *mut c_char {
    let (shift, plaintext) = analysis::frequency_analysis(borrow_str(ciphertext));
    if !out_shift.is_null() {
        *out_shift = shift as c_int;
    }
    into_c_string(plaintext)
}

// A `caesar_brute_force` entry point is intentionally *not* exposed here.
// Brute force is just "decrypt with every shift 1..=25", and serialising 25
// arbitrary plaintexts into one C string cannot be made robust: a plaintext may
// contain any non-letter byte (including the delimiter), so any delimiter-based
// encoding can desync on hostile input. The Dart side composes brute force from
// 25 `caesar_decrypt` calls instead — each still executes in Rust, and the
// result is correct for *any* input with no serialisation caveat.

/// Free a C string previously returned by any function in this library.
/// Passing null is a no-op. Passing a pointer twice, or one this library did
/// not allocate, is undefined behaviour.
///
/// # Safety
/// `s` must be null or a pointer previously returned by this library and not
/// yet freed.
#[no_mangle]
pub unsafe extern "C" fn caesar_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ===================================================================
// Tests — exercise the ABI in-process (round-tripping through raw pointers)
// ===================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Call an `extern "C"` function with a Rust &str and read the result back
    /// into an owned String, freeing the C allocation — mirrors what Dart does.
    unsafe fn roundtrip(f: unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char, s: &str, shift: c_int) -> String {
        let input = CString::new(s).unwrap();
        let out = f(input.as_ptr(), shift);
        let result = CStr::from_ptr(out).to_str().unwrap().to_owned();
        caesar_free_string(out);
        result
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        unsafe {
            assert_eq!(roundtrip(caesar_encrypt, "HELLO", 3), "KHOOR");
            assert_eq!(roundtrip(caesar_decrypt, "KHOOR", 3), "HELLO");
        }
    }

    #[test]
    fn rot13_self_inverse() {
        unsafe {
            let input = CString::new("Hello").unwrap();
            let once = caesar_rot13(input.as_ptr());
            let twice = caesar_rot13(once);
            assert_eq!(CStr::from_ptr(twice).to_str().unwrap(), "Hello");
            caesar_free_string(once);
            caesar_free_string(twice);
        }
    }

    #[test]
    fn null_input_is_empty() {
        unsafe {
            let out = caesar_encrypt(std::ptr::null(), 5);
            assert_eq!(CStr::from_ptr(out).to_str().unwrap(), "");
            caesar_free_string(out);
        }
    }

    #[test]
    fn frequency_analysis_writes_shift() {
        unsafe {
            let ct = CString::new("WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ").unwrap();
            let mut shift: c_int = -1;
            let out = caesar_frequency_analysis(ct.as_ptr(), &mut shift);
            assert_eq!(shift, 3);
            assert_eq!(
                CStr::from_ptr(out).to_str().unwrap(),
                "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
            );
            caesar_free_string(out);
        }
    }

    #[test]
    fn free_null_is_noop() {
        unsafe { caesar_free_string(std::ptr::null_mut()) }
    }
}
