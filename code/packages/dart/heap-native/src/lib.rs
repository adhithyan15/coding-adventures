//! # heap-native — a C ABI over the Rust `heap` crate
//!
//! The *native-through-Rust* half of the Dart `heap` package. The pure crate is
//! generic (`MinHeap<T: Ord>`); a C ABI cannot be generic, so this binding
//! **fixes the element type to `i64`** — the common case for a native
//! priority queue. It demonstrates a new native shape for the campaign: an
//! opaque handle whose operations *return values* (`pop`/`peek`), not just a
//! digest, via a bool-return + out-parameter convention.
//!
//! Two opaque handle types (`MinHeap<i64>`, `MaxHeap<i64>`) plus three
//! array algorithms (`heap_sort`, `nlargest`, `nsmallest`) over `i64` buffers.

use std::os::raw::c_longlong;
use std::slice;

use heap::{nlargest, nsmallest, heap_sort, MaxHeap, MinHeap};

// `c_longlong` is i64 on every platform Dart's `Int64` maps to.
type I64 = c_longlong;

/// Generate the full opaque-handle C ABI for one concrete heap type.
macro_rules! heap_ffi {
    ($ty:ty, $new:ident, $push:ident, $pop:ident, $peek:ident, $len:ident, $empty:ident, $free:ident) => {
        /// Allocate a new heap and return an opaque handle.
        #[no_mangle]
        pub extern "C" fn $new() -> *mut $ty {
            Box::into_raw(Box::new(<$ty>::new()))
        }

        /// Push `value` onto the heap.
        ///
        /// # Safety
        /// `h` must be null or a live handle from the matching `_new`.
        #[no_mangle]
        pub unsafe extern "C" fn $push(h: *mut $ty, value: I64) {
            if !h.is_null() {
                (*h).push(value);
            }
        }

        /// Pop the root into `*out`. Returns `true` if a value was written,
        /// `false` if the heap was empty (or a pointer was null).
        ///
        /// # Safety
        /// `h` null or a live handle; `out` null or writable for one `i64`.
        #[no_mangle]
        pub unsafe extern "C" fn $pop(h: *mut $ty, out: *mut I64) -> bool {
            if h.is_null() || out.is_null() {
                return false;
            }
            match (*h).pop() {
                Some(v) => {
                    *out = v;
                    true
                }
                None => false,
            }
        }

        /// Copy the root into `*out` without removing it. Returns `true` if a
        /// value was written, `false` if the heap was empty.
        ///
        /// # Safety
        /// `h` null or a live handle; `out` null or writable for one `i64`.
        #[no_mangle]
        pub unsafe extern "C" fn $peek(h: *const $ty, out: *mut I64) -> bool {
            if h.is_null() || out.is_null() {
                return false;
            }
            match (*h).peek() {
                Some(v) => {
                    *out = *v;
                    true
                }
                None => false,
            }
        }

        /// Number of elements in the heap (0 for a null handle).
        ///
        /// # Safety
        /// `h` null or a live handle.
        #[no_mangle]
        pub unsafe extern "C" fn $len(h: *const $ty) -> usize {
            if h.is_null() {
                0
            } else {
                (*h).len()
            }
        }

        /// True when the heap is empty (or the handle is null).
        ///
        /// # Safety
        /// `h` null or a live handle.
        #[no_mangle]
        pub unsafe extern "C" fn $empty(h: *const $ty) -> bool {
            if h.is_null() {
                true
            } else {
                (*h).is_empty()
            }
        }

        /// Free a heap handle. Null is a no-op.
        ///
        /// # Safety
        /// `h` null or a live handle not yet freed, unused afterwards.
        #[no_mangle]
        pub unsafe extern "C" fn $free(h: *mut $ty) {
            if !h.is_null() {
                drop(Box::from_raw(h));
            }
        }
    };
}

heap_ffi!(
    MinHeap<I64>,
    heap_min_new,
    heap_min_push,
    heap_min_pop,
    heap_min_peek,
    heap_min_len,
    heap_min_is_empty,
    heap_min_free
);
heap_ffi!(
    MaxHeap<I64>,
    heap_max_new,
    heap_max_push,
    heap_max_pop,
    heap_max_peek,
    heap_max_len,
    heap_max_is_empty,
    heap_max_free
);

// ─── Array algorithms ────────────────────────────────────────────────────────

/// Borrow an input region as an `i64` slice; null or zero length → empty slice.
///
/// # Safety
/// If `len > 0`, `data` must point to `len` readable `i64`s that outlive the call.
unsafe fn in_slice<'a>(data: *const I64, len: usize) -> &'a [I64] {
    if data.is_null() || len == 0 {
        &[]
    } else {
        slice::from_raw_parts(data, len)
    }
}

/// Sort `data[..len]` ascending into the caller-owned `out` buffer (also `len`
/// elements).
///
/// # Safety
/// `data`/`len` a valid region (or 0); `out` writable for `len` `i64`s.
#[no_mangle]
pub unsafe extern "C" fn heap_sort_i64(data: *const I64, len: usize, out: *mut I64) {
    if out.is_null() {
        return;
    }
    let sorted = heap_sort(in_slice(data, len).iter().copied());
    std::ptr::copy_nonoverlapping(sorted.as_ptr(), out, sorted.len());
}

/// Write the `n` largest of `data[..len]` (descending) into `out` and return
/// how many were written (`min(n, len)`).
///
/// # Safety
/// `data`/`len` a valid region (or 0); `out` writable for at least `min(n,len)`
/// `i64`s.
#[no_mangle]
pub unsafe extern "C" fn heap_nlargest_i64(
    data: *const I64,
    len: usize,
    n: usize,
    out: *mut I64,
) -> usize {
    let res = nlargest(in_slice(data, len).iter().copied(), n);
    if !out.is_null() && !res.is_empty() {
        std::ptr::copy_nonoverlapping(res.as_ptr(), out, res.len());
    }
    res.len()
}

/// Write the `n` smallest of `data[..len]` (ascending) into `out` and return
/// how many were written (`min(n, len)`).
///
/// # Safety
/// `data`/`len` a valid region (or 0); `out` writable for at least `min(n,len)`
/// `i64`s.
#[no_mangle]
pub unsafe extern "C" fn heap_nsmallest_i64(
    data: *const I64,
    len: usize,
    n: usize,
    out: *mut I64,
) -> usize {
    let res = nsmallest(in_slice(data, len).iter().copied(), n);
    if !out.is_null() && !res.is_empty() {
        std::ptr::copy_nonoverlapping(res.as_ptr(), out, res.len());
    }
    res.len()
}

// ─── Tests exercising the ABI in-process ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_heap_pops_ascending() {
        unsafe {
            let h = heap_min_new();
            for v in [5, 3, 8, 1, 9] {
                heap_min_push(h, v);
            }
            let mut got = Vec::new();
            let mut out: I64 = 0;
            while heap_min_pop(h, &mut out) {
                got.push(out);
            }
            assert_eq!(got, vec![1, 3, 5, 8, 9]);
            assert!(heap_min_is_empty(h));
            heap_min_free(h);
        }
    }

    #[test]
    fn max_heap_peek_and_len() {
        unsafe {
            let h = heap_max_new();
            heap_max_push(h, 4);
            heap_max_push(h, 9);
            heap_max_push(h, 1);
            let mut out: I64 = 0;
            assert!(heap_max_peek(h, &mut out));
            assert_eq!(out, 9);
            assert_eq!(heap_max_len(h), 3);
            heap_max_free(h);
        }
    }

    #[test]
    fn pop_empty_returns_false() {
        unsafe {
            let h = heap_min_new();
            let mut out: I64 = -1;
            assert!(!heap_min_pop(h, &mut out));
            heap_min_free(h);
        }
    }

    #[test]
    fn array_algorithms() {
        unsafe {
            let data: [I64; 8] = [3, 1, 4, 1, 5, 9, 2, 6];
            let mut sorted = [0i64; 8];
            heap_sort_i64(data.as_ptr(), 8, sorted.as_mut_ptr());
            assert_eq!(sorted, [1, 1, 2, 3, 4, 5, 6, 9]);

            let mut big = [0i64; 3];
            let k = heap_nlargest_i64(data.as_ptr(), 8, 3, big.as_mut_ptr());
            assert_eq!(k, 3);
            assert_eq!(big, [9, 6, 5]);

            let mut small = [0i64; 3];
            let k = heap_nsmallest_i64(data.as_ptr(), 8, 3, small.as_mut_ptr());
            assert_eq!(k, 3);
            assert_eq!(small, [1, 1, 2]);
        }
    }

    #[test]
    fn null_safety() {
        unsafe {
            let mut out: I64 = 0;
            assert!(!heap_min_pop(std::ptr::null_mut(), &mut out));
            assert!(!heap_min_peek(std::ptr::null(), &mut out));
            assert_eq!(heap_min_len(std::ptr::null()), 0);
            assert!(heap_min_is_empty(std::ptr::null()));
            heap_min_free(std::ptr::null_mut());
        }
    }
}
