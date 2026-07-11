//! # sha256-native — a C ABI over the Rust `coding_adventures_sha256` crate
//!
//! The *native-through-Rust* half of the Dart `sha256` package. The pure-Dart
//! port reimplements SHA-256; this crate reuses the audited Rust implementation
//! and exposes it across a stable C ABI for `dart:ffi`.
//!
//! ## Two shapes of C function
//!
//! SHA-256 is *binary* in and 32 bytes out, so — unlike a text cipher — the C
//! ABI here works with byte buffers, not C strings:
//!
//! * **One-shot digest** ([`sha256_digest`]): the caller passes an input
//!   `(ptr, len)` and a **32-byte output buffer** it already owns. We write the
//!   digest into it. No allocation crosses the boundary, so there is nothing to
//!   free.
//! * **Hex** ([`sha256_hex`]): returns a heap C string (freed by
//!   [`sha256_free_string`]) — the one place we allocate.
//! * **Streaming** ([`Sha256Hasher`]): exposed as an **opaque handle**
//!   (`*mut Sha256Hasher`). `new`/`clone` hand out a `Box::into_raw` pointer;
//!   `free` takes it back with `Box::from_raw`. The Dart side attaches a
//!   `NativeFinalizer` so the handle is freed even if the caller forgets.
//!
//! ## Safety contract
//!
//! Input `(ptr, len)` must describe a valid readable region (or `len == 0`).
//! Output buffers passed to the digest functions must be writable for 32 bytes.
//! Each opaque handle from `sha256_hasher_new`/`_clone` must be freed exactly
//! once with `sha256_hasher_free`, and not used afterwards.

use std::ffi::CString;
use std::os::raw::{c_char, c_uchar};
use std::slice;

use coding_adventures_sha256::{sha256 as core_sha256, sha256_hex as core_hex, Sha256Hasher};

/// Borrow an input region as a byte slice. A null pointer or zero length yields
/// an empty slice, so hashing an empty message works and never dereferences
/// null.
///
/// # Safety
/// If `len > 0`, `ptr` must point to at least `len` readable bytes that outlive
/// the call.
unsafe fn slice_from<'a>(ptr: *const c_uchar, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        slice::from_raw_parts(ptr, len)
    }
}

/// Write the 32-byte SHA-256 digest of `data[..len]` into the caller-owned
/// 32-byte buffer `out`.
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`); `out` must be
/// writable for 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha256_digest(data: *const c_uchar, len: usize, out: *mut c_uchar) {
    if out.is_null() {
        return;
    }
    let digest = core_sha256(slice_from(data, len));
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 32);
}

/// Return the 64-character lowercase hex digest of `data[..len]` as a newly
/// allocated C string; free it with [`sha256_free_string`].
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`).
#[no_mangle]
pub unsafe extern "C" fn sha256_hex(data: *const c_uchar, len: usize) -> *mut c_char {
    // Hex output is ASCII with no interior NUL, so CString::new never fails.
    CString::new(core_hex(slice_from(data, len)))
        .unwrap_or_default()
        .into_raw()
}

/// Free a C string returned by [`sha256_hex`]. Null is a no-op.
///
/// # Safety
/// `s` must be null or a pointer from [`sha256_hex`] not yet freed.
#[no_mangle]
pub unsafe extern "C" fn sha256_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ─── Streaming hasher (opaque handle) ────────────────────────────────────────

/// Allocate a new streaming hasher and return an opaque handle.
#[no_mangle]
pub extern "C" fn sha256_hasher_new() -> *mut Sha256Hasher {
    Box::into_raw(Box::new(Sha256Hasher::new()))
}

/// Feed `data[..len]` into the hasher `h`.
///
/// # Safety
/// `h` must be null or a live handle; `data`/`len` a valid region (or 0).
#[no_mangle]
pub unsafe extern "C" fn sha256_hasher_update(
    h: *mut Sha256Hasher,
    data: *const c_uchar,
    len: usize,
) {
    if h.is_null() {
        return;
    }
    (*h).update(slice_from(data, len));
}

/// Write the current 32-byte digest of hasher `h` into the caller-owned 32-byte
/// buffer `out`. Non-destructive: the hasher can keep receiving updates.
///
/// # Safety
/// `h` must be null or a live handle; `out` writable for 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha256_hasher_digest(h: *const Sha256Hasher, out: *mut c_uchar) {
    if h.is_null() || out.is_null() {
        return;
    }
    let digest = (*h).digest();
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 32);
}

/// Return an independent copy of hasher `h` as a new opaque handle (or null if
/// `h` is null). Hashing either copy afterwards does not affect the other.
///
/// # Safety
/// `h` must be null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn sha256_hasher_clone(h: *const Sha256Hasher) -> *mut Sha256Hasher {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*h).clone_hasher()))
}

/// Free a hasher handle from `sha256_hasher_new`/`_clone`. Null is a no-op.
///
/// # Safety
/// `h` must be null or a live handle not yet freed, and unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn sha256_hasher_free(h: *mut Sha256Hasher) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

// ─── Tests exercising the ABI in-process ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn hex_of(digest: &[u8; 32]) -> String {
        digest.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[test]
    fn digest_writes_32_bytes() {
        let mut out = [0u8; 32];
        unsafe { sha256_digest(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(hex_of(&out), ABC);
    }

    #[test]
    fn digest_empty_via_null() {
        let mut out = [0u8; 32];
        unsafe { sha256_digest(std::ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(
            hex_of(&out),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hex_matches() {
        unsafe {
            let p = sha256_hex(b"abc".as_ptr(), 3);
            let s = std::ffi::CStr::from_ptr(p).to_str().unwrap().to_owned();
            sha256_free_string(p);
            assert_eq!(s, ABC);
        }
    }

    #[test]
    fn streaming_matches_oneshot_and_clone_is_independent() {
        unsafe {
            let h = sha256_hasher_new();
            sha256_hasher_update(h, b"ab".as_ptr(), 2);
            let h2 = sha256_hasher_clone(h);
            sha256_hasher_update(h2, b"c".as_ptr(), 1);
            sha256_hasher_update(h, b"x".as_ptr(), 1);

            let mut d2 = [0u8; 32];
            sha256_hasher_digest(h2, d2.as_mut_ptr());
            assert_eq!(hex_of(&d2), ABC); // "abc"

            let mut d1 = [0u8; 32];
            sha256_hasher_digest(h, d1.as_mut_ptr());
            let mut expect_abx = [0u8; 32];
            sha256_digest(b"abx".as_ptr(), 3, expect_abx.as_mut_ptr());
            assert_eq!(d1, expect_abx);

            sha256_hasher_free(h);
            sha256_hasher_free(h2);
        }
    }

    #[test]
    fn null_handle_is_safe() {
        unsafe {
            sha256_hasher_update(std::ptr::null_mut(), b"x".as_ptr(), 1);
            let mut out = [0u8; 32];
            sha256_hasher_digest(std::ptr::null(), out.as_mut_ptr());
            assert!(sha256_hasher_clone(std::ptr::null()).is_null());
            sha256_hasher_free(std::ptr::null_mut());
        }
    }
}
