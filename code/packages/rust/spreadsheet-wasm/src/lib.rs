//! # spreadsheet-wasm
//!
//! The thin `extern "C"` + linear-memory ABI that lets a JavaScript host drive
//! [`spreadsheet_core_wasm::SpreadsheetSession`] after this crate is compiled to
//! `wasm32-unknown-unknown`. It is the *boundary*, not the logic: it owns
//! string marshalling across linear memory and nothing else. All spreadsheet
//! behaviour lives below it (`spreadsheet-core-wasm` → `spreadsheet-core`).
//!
//! This is the repo's zero-dependency WASM convention — no `wasm-bindgen`, no
//! third-party FFI framework — just hand-written `#[no_mangle] extern "C"`
//! exports and a tiny memory protocol the JS loader mirrors.
//!
//! ## Memory protocol
//!
//! WASM linear memory is a flat byte array shared with JS. Strings cross it as
//! `(ptr, len)` pairs:
//!
//! - **JS → WASM (inputs):** JS calls [`alloc`]`(len)`, writes `len` UTF-8
//!   bytes at the returned pointer, passes `(ptr, len)`, and frees the buffer
//!   with [`dealloc`]`(ptr, len)` after the call returns.
//! - **WASM → JS (outputs):** each string-returning export allocates a buffer
//!   laid out as `[len: u32 little-endian][len bytes of UTF-8]` and returns its
//!   pointer. JS reads the 4-byte length, then the bytes, then frees the whole
//!   buffer with [`dealloc`]`(ptr, 4 + len)`.
//!
//! Every allocation uses an explicit [`Layout`] with `align = 1`, and `dealloc`
//! rebuilds the *same* layout from the `len` JS passes back — so there is never
//! a capacity mismatch between allocation and free (the classic source of
//! unsoundness in hand-rolled WASM ABIs).
//!
//! ## Session model
//!
//! A single global [`SpreadsheetSession`] lives in thread-local storage (WASM
//! is single-threaded). [`reset`] starts a fresh sheet; the JS loader calls it
//! from its `createSpreadsheet()` so each new workbook begins clean. One live
//! workbook at a time is all the VisiCalc demos need.

use std::alloc::{alloc as raw_alloc, dealloc as raw_dealloc, Layout};
use std::cell::RefCell;

use spreadsheet_core_wasm::SpreadsheetSession;

thread_local! {
    static SESSION: RefCell<SpreadsheetSession> = RefCell::new(SpreadsheetSession::new());
}

// ---------------------------------------------------------------------------
// Linear-memory allocator exports.
// ---------------------------------------------------------------------------

/// Allocate `len` bytes of linear memory and return a pointer to them. Returns
/// null for a zero-length request. Pair with exactly one [`dealloc`]`(ptr,
/// len)`.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    // align = 1: these buffers hold UTF-8 bytes with no alignment needs. A
    // request so large the layout is invalid (> isize::MAX) returns null
    // rather than panicking — a panic would trap the whole WASM module.
    let layout = match Layout::from_size_align(len, 1) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    // SAFETY: `layout` has non-zero size; a null return is handled by the
    // caller (JS checks for 0). No other invariants.
    unsafe { raw_alloc(layout) }
}

/// Free a buffer previously returned by [`alloc`] or by a string-returning
/// export (in which case `len` is `4 + payload_length`).
///
/// # Safety
/// `ptr` and `len` must exactly match a prior allocation made by this module
/// and not yet freed. This holds when the JS loader follows the documented
/// protocol; it is the loader's responsibility, mirrored here.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let layout = match Layout::from_size_align(len, 1) {
        Ok(l) => l,
        Err(_) => return,
    };
    raw_dealloc(ptr, layout);
}

// ---------------------------------------------------------------------------
// Marshalling helpers (private).
// ---------------------------------------------------------------------------

/// Read a `(ptr, len)` input as an owned `String`. Invalid UTF-8 is replaced
/// (lossy) rather than panicking — the host should never send invalid UTF-8,
/// but the ABI stays total if it does.
///
/// # Safety
/// `ptr` must point to `len` readable bytes (or be null with `len == 0`).
unsafe fn read_input(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf8_lossy(slice).into_owned()
}

/// Pack a result string into a freshly allocated `[len: u32 LE][bytes]` buffer
/// and return its pointer. The caller (JS) frees it with `dealloc(ptr, 4 +
/// len)`.
fn pack(s: String) -> *mut u8 {
    let bytes = s.into_bytes();
    let payload_len = bytes.len();
    // Guard the length prefix add and the layout the same way `alloc` does:
    // return null on overflow / invalid layout rather than panicking (a panic
    // traps the module) — and never let a wrapped `total` produce a buffer
    // smaller than the bytes we then copy into it.
    let total = match payload_len.checked_add(4) {
        Some(t) => t,
        None => return std::ptr::null_mut(),
    };
    let layout = match Layout::from_size_align(total, 1) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };
    // SAFETY: `total` ≥ 4 > 0, so the layout has non-zero size. We write
    // exactly `total` bytes (4-byte length prefix + payload) into the freshly
    // allocated region before returning it; nothing else aliases it.
    unsafe {
        let ptr = raw_alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        let len_prefix = (payload_len as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_prefix.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), payload_len);
        ptr
    }
}

// ---------------------------------------------------------------------------
// Session exports — each mirrors a `SpreadsheetSession` method.
// ---------------------------------------------------------------------------

/// Replace the global session with a fresh, empty one.
#[no_mangle]
pub extern "C" fn reset() {
    SESSION.with(|s| *s.borrow_mut() = SpreadsheetSession::new());
}

/// `set_cell(a1, raw)` → JSON status string. See
/// [`SpreadsheetSession::set_cell`].
///
/// # Safety
/// The `(ptr, len)` pairs must describe readable byte ranges (see
/// [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn set_cell(
    a1_ptr: *const u8,
    a1_len: usize,
    raw_ptr: *const u8,
    raw_len: usize,
) -> *mut u8 {
    let a1 = read_input(a1_ptr, a1_len);
    let raw = read_input(raw_ptr, raw_len);
    let out = SESSION.with(|s| s.borrow_mut().set_cell(&a1, &raw));
    pack(out)
}

/// `get_value(a1)` → JSON value object. See [`SpreadsheetSession::get_value`].
///
/// # Safety
/// `(ptr, len)` must describe a readable byte range.
#[no_mangle]
pub unsafe extern "C" fn get_value(a1_ptr: *const u8, a1_len: usize) -> *mut u8 {
    let a1 = read_input(a1_ptr, a1_len);
    pack(SESSION.with(|s| s.borrow().get_value(&a1)))
}

/// `get_raw(a1)` → the typed source string. See
/// [`SpreadsheetSession::get_raw`].
///
/// # Safety
/// `(ptr, len)` must describe a readable byte range.
#[no_mangle]
pub unsafe extern "C" fn get_raw(a1_ptr: *const u8, a1_len: usize) -> *mut u8 {
    let a1 = read_input(a1_ptr, a1_len);
    pack(SESSION.with(|s| s.borrow().get_raw(&a1)))
}

/// `get_values()` → JSON map of every set cell. See
/// [`SpreadsheetSession::get_values`].
#[no_mangle]
pub extern "C" fn get_values() -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().get_values()))
}

// ---------------------------------------------------------------------------
// Host-target tests: exercise the ABI exactly as the JS loader would, but in
// Rust (so they run under `cargo test` with no WASM toolchain). We drive the
// `(ptr, len)` protocol by hand to prove the marshalling and the alloc/dealloc
// pairing are sound.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Copy a Rust string into module memory via `alloc`, returning the
    /// `(ptr, len)` the ABI expects — the JS `writeStr` in Rust.
    fn put(s: &str) -> (*mut u8, usize) {
        let bytes = s.as_bytes();
        let ptr = alloc(bytes.len());
        // SAFETY: `alloc` reserved `bytes.len()` writable bytes at `ptr`.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        (ptr, bytes.len())
    }

    /// Read a `[len][bytes]` result buffer and free it — the JS `readResult`.
    fn take(ptr: *mut u8) -> String {
        // SAFETY: `ptr` points at a buffer produced by `pack`.
        unsafe {
            let len = u32::from_le_bytes([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]) as usize;
            let bytes = std::slice::from_raw_parts(ptr.add(4), len).to_vec();
            dealloc(ptr, 4 + len);
            String::from_utf8(bytes).unwrap()
        }
    }

    fn set(a1: &str, raw: &str) -> String {
        let (ap, al) = put(a1);
        let (rp, rl) = put(raw);
        // SAFETY: both pairs come from `put`/`alloc`.
        let out = unsafe { set_cell(ap, al, rp, rl) };
        unsafe {
            dealloc(ap, al);
            dealloc(rp, rl);
        }
        take(out)
    }

    fn value(a1: &str) -> String {
        let (ap, al) = put(a1);
        let out = unsafe { get_value(ap, al) };
        unsafe { dealloc(ap, al) };
        take(out)
    }

    fn raw(a1: &str) -> String {
        let (ap, al) = put(a1);
        let out = unsafe { get_raw(ap, al) };
        unsafe { dealloc(ap, al) };
        take(out)
    }

    #[test]
    fn abi_round_trips_a_formula() {
        reset();
        assert_eq!(set("B1", "15"), r#"{"ok":true}"#);
        set("B2", "8");
        set("B3", "12");
        set("B4", "4");
        set("B5", "7");
        set("B6", "=SUM(B1:B5)");
        assert_eq!(value("B6"), r#"{"kind":"number","value":46.0}"#);
        assert_eq!(raw("B6"), "=SUM(B1:B5)");
        // Recalc on dependency change.
        set("B1", "115");
        assert_eq!(value("B6"), r#"{"kind":"number","value":146.0}"#);
    }

    #[test]
    fn abi_get_values_and_reset() {
        reset();
        set("A1", "2");
        set("A2", "=A1*3");
        let vals = take(get_values());
        assert!(vals.contains(r#""A1":{"kind":"number","value":2.0}"#), "{vals}");
        assert!(vals.contains(r#""A2":{"kind":"number","value":6.0}"#), "{vals}");
        reset();
        assert_eq!(take(get_values()), "{}");
    }

    #[test]
    fn abi_empty_and_null_inputs_are_safe() {
        reset();
        // Zero-length input → alloc returns null, read_input handles it.
        // SAFETY: null ptr with len 0 is the documented empty case.
        let out = unsafe { get_value(std::ptr::null(), 0) };
        // An empty A1 fails to parse → error object, not a crash.
        assert!(take(out).contains(r#""kind":"error""#));
        // dealloc(null, 0) is a no-op.
        unsafe { dealloc(std::ptr::null_mut(), 0) };
    }

    #[test]
    fn abi_error_value_round_trips() {
        reset();
        set("A1", "=1/0");
        assert_eq!(value("A1"), r##"{"code":"#DIV/0!","kind":"error"}"##);
    }
}
