//! # sha1-native — a C ABI over the Rust `coding_adventures_sha1` crate
//!
//! The *native-through-Rust* half of the Dart `sha1` package. Same shape as
//! `sha256-native`/`md5-native`: binary byte buffers, a caller-owned digest
//! buffer (20 bytes), and an opaque streaming handle.
//!
//! SHA-1 is broken for collision resistance; this binding exists for parity and
//! checksum/legacy use, not security.

use std::ffi::CString;
use std::os::raw::{c_char, c_uchar};
use std::slice;

use coding_adventures_sha1::{hex_string as core_hex, sum1 as core_sha1, Digest};

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

/// Write the 20-byte SHA-1 digest of `data[..len]` into the caller-owned 20-byte
/// buffer `out`.
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`); `out` writable for
/// 20 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha1_digest(data: *const c_uchar, len: usize, out: *mut c_uchar) {
    if out.is_null() {
        return;
    }
    let digest = core_sha1(slice_from(data, len));
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 20);
}

/// Return the 40-character lowercase hex digest of `data[..len]` as a newly
/// allocated C string; free it with [`sha1_free_string`].
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`).
#[no_mangle]
pub unsafe extern "C" fn sha1_hex(data: *const c_uchar, len: usize) -> *mut c_char {
    CString::new(core_hex(slice_from(data, len)))
        .unwrap_or_default()
        .into_raw()
}

/// Free a C string returned by [`sha1_hex`]. Null is a no-op.
///
/// # Safety
/// `s` must be null or a pointer from [`sha1_hex`] not yet freed.
#[no_mangle]
pub unsafe extern "C" fn sha1_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ─── Streaming digest (opaque handle) ────────────────────────────────────────

/// Allocate a new streaming SHA-1 hasher and return an opaque handle.
#[no_mangle]
pub extern "C" fn sha1_hasher_new() -> *mut Digest {
    Box::into_raw(Box::new(Digest::new()))
}

/// Feed `data[..len]` into the hasher `h`.
///
/// # Safety
/// `h` null or a live handle; `data`/`len` a valid region (or 0).
#[no_mangle]
pub unsafe extern "C" fn sha1_hasher_update(h: *mut Digest, data: *const c_uchar, len: usize) {
    if h.is_null() {
        return;
    }
    (*h).update(slice_from(data, len));
}

/// Write the current 20-byte digest of hasher `h` into the caller-owned 20-byte
/// buffer `out`. Non-destructive.
///
/// # Safety
/// `h` null or a live handle; `out` writable for 20 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha1_hasher_digest(h: *const Digest, out: *mut c_uchar) {
    if h.is_null() || out.is_null() {
        return;
    }
    let digest = (*h).sum1();
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 20);
}

/// Return an independent copy of hasher `h` (or null if `h` is null).
///
/// # Safety
/// `h` null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn sha1_hasher_clone(h: *const Digest) -> *mut Digest {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*h).clone_digest()))
}

/// Free a hasher handle from `sha1_hasher_new`/`_clone`. Null is a no-op.
///
/// # Safety
/// `h` must be null or a live handle not yet freed, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn sha1_hasher_free(h: *mut Digest) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

// ─── Tests exercising the ABI in-process ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";
    const EMPTY: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

    fn hex_of(d: &[u8; 20]) -> String {
        d.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[test]
    fn digest_writes_20_bytes() {
        let mut out = [0u8; 20];
        unsafe { sha1_digest(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(hex_of(&out), ABC);
    }

    #[test]
    fn digest_empty_via_null() {
        let mut out = [0u8; 20];
        unsafe { sha1_digest(std::ptr::null(), 0, out.as_mut_ptr()) };
        assert_eq!(hex_of(&out), EMPTY);
    }

    #[test]
    fn hex_matches() {
        unsafe {
            let p = sha1_hex(b"abc".as_ptr(), 3);
            let s = std::ffi::CStr::from_ptr(p).to_str().unwrap().to_owned();
            sha1_free_string(p);
            assert_eq!(s, ABC);
        }
    }

    #[test]
    fn streaming_matches_oneshot_and_clone_is_independent() {
        unsafe {
            let h = sha1_hasher_new();
            sha1_hasher_update(h, b"ab".as_ptr(), 2);
            let h2 = sha1_hasher_clone(h);
            sha1_hasher_update(h2, b"c".as_ptr(), 1);
            sha1_hasher_update(h, b"x".as_ptr(), 1);

            let mut d2 = [0u8; 20];
            sha1_hasher_digest(h2, d2.as_mut_ptr());
            assert_eq!(hex_of(&d2), ABC); // "abc"

            let mut d1 = [0u8; 20];
            sha1_hasher_digest(h, d1.as_mut_ptr());
            let mut expect_abx = [0u8; 20];
            sha1_digest(b"abx".as_ptr(), 3, expect_abx.as_mut_ptr());
            assert_eq!(d1, expect_abx);

            sha1_hasher_free(h);
            sha1_hasher_free(h2);
        }
    }

    #[test]
    fn null_handle_is_safe() {
        unsafe {
            sha1_hasher_update(std::ptr::null_mut(), b"x".as_ptr(), 1);
            let mut out = [0u8; 20];
            sha1_hasher_digest(std::ptr::null(), out.as_mut_ptr());
            assert!(sha1_hasher_clone(std::ptr::null()).is_null());
            sha1_hasher_free(std::ptr::null_mut());
        }
    }
}
