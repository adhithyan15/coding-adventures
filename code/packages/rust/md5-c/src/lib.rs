//! # md5-c — C ABI wrapper for the `coding_adventures_md5` crate
//!
//! Exposes MD5 over a stable C ABI (static + dynamic library) for Swift/C/C++
//! compile-time linking. Mirrors `sha256-c`: 16-byte digest written into a
//! caller-owned buffer, plus an opaque streaming handle.
//!
//! MD5 is cryptographically broken — checksum use only.

use std::os::raw::c_uchar;
use std::slice;

use coding_adventures_md5::{sum_md5, Digest};

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
/// `data`/`len` a valid region (or 0); `out` writable for 16 bytes.
#[no_mangle]
pub unsafe extern "C" fn md5_c_digest(data: *const c_uchar, len: usize, out: *mut c_uchar) {
    if out.is_null() {
        return;
    }
    let digest = sum_md5(slice_from(data, len));
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 16);
}

/// Allocate a new streaming hasher and return an opaque handle.
#[no_mangle]
pub extern "C" fn md5_c_hasher_new() -> *mut Digest {
    Box::into_raw(Box::new(Digest::new()))
}

/// Feed `data[..len]` into the hasher `h`.
///
/// # Safety
/// `h` null or a live handle; `data`/`len` a valid region (or 0).
#[no_mangle]
pub unsafe extern "C" fn md5_c_hasher_update(h: *mut Digest, data: *const c_uchar, len: usize) {
    if h.is_null() {
        return;
    }
    (*h).update(slice_from(data, len));
}

/// Write the current 16-byte digest of hasher `h` into `out` (non-destructive).
///
/// # Safety
/// `h` null or a live handle; `out` writable for 16 bytes.
#[no_mangle]
pub unsafe extern "C" fn md5_c_hasher_digest(h: *const Digest, out: *mut c_uchar) {
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
pub unsafe extern "C" fn md5_c_hasher_clone(h: *const Digest) -> *mut Digest {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*h).clone_digest()))
}

/// Free a hasher handle. Null is a no-op.
///
/// # Safety
/// `h` null or a live handle not yet freed, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn md5_c_hasher_free(h: *mut Digest) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ABC: &str = "900150983cd24fb0d6963f7d28e17f72";
    fn hex(d: &[u8; 16]) -> String {
        d.iter().map(|b| format!("{:02x}", b)).collect()
    }
    #[test]
    fn digest_matches() {
        let mut out = [0u8; 16];
        unsafe { md5_c_digest(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(hex(&out), ABC);
    }
    #[test]
    fn streaming_and_clone() {
        unsafe {
            let h = md5_c_hasher_new();
            md5_c_hasher_update(h, b"ab".as_ptr(), 2);
            let h2 = md5_c_hasher_clone(h);
            md5_c_hasher_update(h2, b"c".as_ptr(), 1);
            let mut d = [0u8; 16];
            md5_c_hasher_digest(h2, d.as_mut_ptr());
            assert_eq!(hex(&d), ABC);
            md5_c_hasher_free(h);
            md5_c_hasher_free(h2);
        }
    }
    #[test]
    fn null_safe() {
        unsafe {
            md5_c_hasher_update(std::ptr::null_mut(), b"x".as_ptr(), 1);
            let mut out = [0u8; 16];
            md5_c_hasher_digest(std::ptr::null(), out.as_mut_ptr());
            assert!(md5_c_hasher_clone(std::ptr::null()).is_null());
            md5_c_hasher_free(std::ptr::null_mut());
        }
    }
}
