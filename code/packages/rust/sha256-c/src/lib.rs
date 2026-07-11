//! # sha256-c — C ABI wrapper for the `coding_adventures_sha256` crate
//!
//! This crate exposes SHA-256 over a stable C ABI so that Swift, C, and C++
//! callers can link against the compiled **static** library (`libsha256_c.a`)
//! at compile time — or the dynamic library for runtime loading.
//!
//! It mirrors the C-ABI style of `gf256-c`: binary in, fixed-size bytes out,
//! plus an opaque streaming handle. Nothing panics across the C boundary, and
//! the digest functions write into a **caller-owned buffer** so no allocation
//! crosses the boundary on the one-shot path.
//!
//! ## The contract (see `sha256_c.h`)
//!
//! ```c
//! void  sha256_c_digest(const uint8_t* data, size_t len, uint8_t* out32);
//! HASHER* sha256_c_hasher_new(void);
//! void    sha256_c_hasher_update(HASHER*, const uint8_t* data, size_t len);
//! void    sha256_c_hasher_digest(const HASHER*, uint8_t* out32);
//! HASHER* sha256_c_hasher_clone(const HASHER*);
//! void    sha256_c_hasher_free(HASHER*);
//! ```
//!
//! The streaming handle is an opaque `*mut Sha256Hasher`: `new`/`clone` hand out
//! a `Box::into_raw` pointer and `free` reclaims it with `Box::from_raw`. Hex is
//! intentionally not exposed — callers format the 32 digest bytes themselves,
//! so there is no C-string allocation to free.

use std::os::raw::c_uchar;
use std::slice;

use coding_adventures_sha256::{sha256 as core_sha256, Sha256Hasher};

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

/// Write the 32-byte SHA-256 digest of `data[..len]` into the caller-owned
/// 32-byte buffer `out`.
///
/// # Safety
/// `data`/`len` must describe a valid region (or `len == 0`); `out` must be
/// writable for 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha256_c_digest(data: *const c_uchar, len: usize, out: *mut c_uchar) {
    if out.is_null() {
        return;
    }
    let digest = core_sha256(slice_from(data, len));
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 32);
}

/// Allocate a new streaming hasher and return an opaque handle.
#[no_mangle]
pub extern "C" fn sha256_c_hasher_new() -> *mut Sha256Hasher {
    Box::into_raw(Box::new(Sha256Hasher::new()))
}

/// Feed `data[..len]` into the hasher `h`.
///
/// # Safety
/// `h` must be null or a live handle; `data`/`len` a valid region (or 0).
#[no_mangle]
pub unsafe extern "C" fn sha256_c_hasher_update(
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
/// buffer `out`. Non-destructive.
///
/// # Safety
/// `h` must be null or a live handle; `out` writable for 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn sha256_c_hasher_digest(h: *const Sha256Hasher, out: *mut c_uchar) {
    if h.is_null() || out.is_null() {
        return;
    }
    let digest = (*h).digest();
    std::ptr::copy_nonoverlapping(digest.as_ptr(), out, 32);
}

/// Return an independent copy of hasher `h` (or null if `h` is null).
///
/// # Safety
/// `h` must be null or a live handle.
#[no_mangle]
pub unsafe extern "C" fn sha256_c_hasher_clone(h: *const Sha256Hasher) -> *mut Sha256Hasher {
    if h.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*h).clone_hasher()))
}

/// Free a hasher handle from `sha256_c_hasher_new`/`_clone`. Null is a no-op.
///
/// # Safety
/// `h` must be null or a live handle not yet freed, unused afterwards.
#[no_mangle]
pub unsafe extern "C" fn sha256_c_hasher_free(h: *mut Sha256Hasher) {
    if !h.is_null() {
        drop(Box::from_raw(h));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn hex(d: &[u8; 32]) -> String {
        d.iter().map(|b| format!("{:02x}", b)).collect()
    }

    #[test]
    fn digest_matches() {
        let mut out = [0u8; 32];
        unsafe { sha256_c_digest(b"abc".as_ptr(), 3, out.as_mut_ptr()) };
        assert_eq!(hex(&out), ABC);
    }

    #[test]
    fn streaming_matches_and_clone_independent() {
        unsafe {
            let h = sha256_c_hasher_new();
            sha256_c_hasher_update(h, b"ab".as_ptr(), 2);
            let h2 = sha256_c_hasher_clone(h);
            sha256_c_hasher_update(h2, b"c".as_ptr(), 1);
            let mut d = [0u8; 32];
            sha256_c_hasher_digest(h2, d.as_mut_ptr());
            assert_eq!(hex(&d), ABC);
            sha256_c_hasher_free(h);
            sha256_c_hasher_free(h2);
        }
    }

    #[test]
    fn null_is_safe() {
        unsafe {
            sha256_c_hasher_update(std::ptr::null_mut(), b"x".as_ptr(), 1);
            let mut out = [0u8; 32];
            sha256_c_hasher_digest(std::ptr::null(), out.as_mut_ptr());
            assert!(sha256_c_hasher_clone(std::ptr::null()).is_null());
            sha256_c_hasher_free(std::ptr::null_mut());
        }
    }
}
