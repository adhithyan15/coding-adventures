//! # md5-native — a C ABI over the Rust `coding_adventures_md5` crate
//!
//! The *native-through-Rust* half of the Dart `md5` package. The pure-Dart port
//! reimplements MD5; this crate reuses the Rust implementation and exposes it
//! across a stable C ABI for `dart:ffi`. It follows the same shape as
//! `sha256-native`: binary byte buffers, a caller-owned digest buffer, and an
//! opaque streaming handle.
//!
//! * **One-shot digest** ([`md5_digest`]): caller passes input `(ptr, len)` and
//!   a **16-byte output buffer** it owns; we write the digest into it. No
//!   allocation crosses the boundary.
//! * **Hex** ([`md5_hex`]): returns a heap C string (freed by
//!   [`md5_free_string`]).
//! * **Streaming** ([`Digest`]): an opaque handle (`*mut Digest`) with
//!   new/update/digest/clone/free.
//!
//! MD5 is cryptographically broken; this binding exists for parity and checksum
//! use, not security.

use std::ffi::CString;
use std::os::raw::{c_char, c_uchar};
use std::slice;

use coding_adventures_md5::{hex_string as core_hex, sum_md5 as core_md5, Digest};

/// Borrow an input region as a byte slice; null or zero length yields an empty
/// slice.
///
/// # Safety
/// If `len > 0`, `ptr` must point to `len` readable bytes that outlive the call.
unsafe fn slice_from<'a>(ptr: *const c_uchar, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        slice::from_raw_parts(ptr, len)
    }
}

/// Write the 16-byte MD5 digest of `data[..len]` into the caller-owned 16-byte
/// buffer `out`.
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`); `out` writable for
/// 16 bytes.
#[no_mangle]
pub unsafe extern "C" fn md5_digest(data: *const c_uchar, len: usize, out: *mut c_uchar) {
    if out.is_null() {
        return;
    }
    let digest = core_md5(slice_from(data, len));
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 16);
}

/// Return the 32-character lowercase hex digest of `data[..len]` as a newly
/// allocated C string; free it with [`md5_free_string`].
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`).
#[no_mangle]
pub unsafe extern "C" fn md5_hex(data: *const c_uchar, len: usize) -> *mut c_char {
    CString::new(core_hex(slice_from(data, len)))
        .unwrap_or_default()
        .into_raw()
}

/// Free a C string returned by [`md5_hex`]. Null is a no-op.
///
/// # Safety
/// `s` must be null or a pointer from [`md5_hex`] not yet freed.
#[no_mangle]
pub unsafe extern "C" fn md5_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ─── Streaming digest (opaque handle) ────────────────────────────────────────

/// Allocate a new streaming MD5 hasher and return an opaque handle.
#[no_mangle]
pub extern "C" fn md5_hasher_new() -> *mut Digest {
    Box::into_raw(Box::new(Digest::new()))
}

/// Feed `data[..len]` into the hasher `h`.
///
/// # Safety
/// `h` null or a live handle; `data`/`len` a valid region (or 0).
#[no_mangle]
pub unsafe extern "C" fn md5_hasher_update(h: *mut Digest, data: *const c_uchar, len: usize) {
    if h.is_null() {
        return;
    }
    (*h).update(slice_from(data, len));
}

/// Write the current 16-byte digest of hasher `h` into the caller-owned 16-byte
/// buffer `out`. Non-destructive.
///
/// # Safety
/// `h` null or a live handle; `out` writable for 16 bytes.
#[no_mangle]
pub unsafe extern "C" fn md5_hasher_digest(h: *const Digest, out: *mut c_uchar) {
    if h.is_null() || out.is_null() {
        return;
    }
    let digest = (*h).sum_md5();
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 16);
}

/// Return an independent copy of hasher `h` (or null if `h` is null).
///
/// # Safety
/// `h` null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn md5_hasher_clone(h: *const Digest) -> *mut Digest {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*h).clone_digest()))
}

/// Free a hasher handle from `md5_hasher_new`/`_clone`. Null is a no-op.
///
/// # Safety
/// `h` must be null or a live handle not yet freed, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn md5_hasher_free(h: *mut Digest) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

// ─── Tests exercising the ABI in-process ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "900150983cd24fb0d6963f7d28e17f72";
    const EMPTY: &str = "d41d8cd98f00b204e9800998ecf8427e";

    fn hex_of(d: &[u8; 16]) -> String {
        d.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[test]
    fn digest_writes_16_bytes() {
        let mut out = [0u8; 16];
        unsafe { md5_digest(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(hex_of(&out), ABC);
    }

    #[test]
    fn digest_empty_via_null() {
        let mut out = [0u8; 16];
        unsafe { md5_digest(std::ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(hex_of(&out), EMPTY);
    }

    #[test]
    fn hex_matches() {
        unsafe {
            let p = md5_hex(b"abc".as_ptr(), 3);
            let s = std::ffi::CStr::from_ptr(p).to_str().unwrap().to_owned();
            md5_free_string(p);
            assert_eq!(s, ABC);
        }
    }

    #[test]
    fn streaming_matches_oneshot_and_clone_is_independent() {
        unsafe {
            let h = md5_hasher_new();
            md5_hasher_update(h, b"ab".as_ptr(), 2);
            let h2 = md5_hasher_clone(h);
            md5_hasher_update(h2, b"c".as_ptr(), 1);
            md5_hasher_update(h, b"x".as_ptr(), 1);

            let mut d2 = [0u8; 16];
            md5_hasher_digest(h2, d2.as_mut_ptr());
            assert_eq!(hex_of(&d2), ABC); // "abc"

            let mut d1 = [0u8; 16];
            md5_hasher_digest(h, d1.as_mut_ptr());
            let mut expect_abx = [0u8; 16];
            md5_digest(b"abx".as_ptr(), 3, expect_abx.as_mut_ptr());
            assert_eq!(d1, expect_abx);

            md5_hasher_free(h);
            md5_hasher_free(h2);
        }
    }

    #[test]
    fn null_handle_is_safe() {
        unsafe {
            md5_hasher_update(std::ptr::null_mut(), b"x".as_ptr(), 1);
            let mut out = [0u8; 16];
            md5_hasher_digest(std::ptr::null(), out.as_mut_ptr());
            assert!(md5_hasher_clone(std::ptr::null()).is_null());
            md5_hasher_free(std::ptr::null_mut());
        }
    }
}
