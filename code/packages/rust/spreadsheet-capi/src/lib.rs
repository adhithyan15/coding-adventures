//! # spreadsheet-capi
//!
//! A **stable C ABI** over [`spreadsheet_core_wasm::SpreadsheetSession`], so the
//! spreadsheet engine can be linked into the native VisiCalc demos:
//!
//! | Platform        | how it calls this library |
//! |-----------------|---------------------------|
//! | Qt / C++        | the C header directly     |
//! | SwiftUI / Swift | C interop (module map)    |
//! | Compose/Android | JNI → these C functions   |
//! | Flutter / Dart  | `dart:ffi`                |
//! | XAML / .NET     | P/Invoke                  |
//!
//! It is the native sibling of `spreadsheet-wasm` (which exposes the same
//! engine over a WASM linear-memory ABI for the browser). Both are thin
//! boundaries over the shared facade; the spreadsheet logic lives below in
//! `spreadsheet-core`.
//!
//! ## Contract
//!
//! - [`sc_session_new`] returns an opaque handle; free it with
//!   [`sc_session_free`].
//! - Every `sc_*` call that returns a `char *` returns a heap-allocated,
//!   NUL-terminated UTF-8 string (JSON for values, raw text for `sc_get_raw`).
//!   **The caller owns it and must free it with [`sc_string_free`]** — never
//!   `free()` it directly (it was not allocated by the C allocator). A NULL
//!   return signals an error (e.g. a null handle).
//! - The JSON value shape matches the TypeScript and WASM engines exactly, so
//!   every frontend parses identical output.
//!
//! ## Safety model
//!
//! The pointers crossing this boundary are the caller's responsibility (the
//! standard C-FFI contract, documented per function). The engine itself is
//! hardened against adversarial formulas, and the facade's `set_cell` already
//! catches panics — so a malformed cell string yields an error *value*, never
//! an unwind across the FFI boundary. Reads are panic-free by construction.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use spreadsheet_core_wasm::SpreadsheetSession;

/// Opaque session handle. The C side only ever sees `ScSession *`.
pub struct ScSession {
    inner: SpreadsheetSession,
}

/// Create a new, empty spreadsheet session. Free it with [`sc_session_free`].
#[no_mangle]
pub extern "C" fn sc_session_new() -> *mut ScSession {
    Box::into_raw(Box::new(ScSession {
        inner: SpreadsheetSession::new(),
    }))
}

/// Free a session created by [`sc_session_new`]. Safe to call with NULL.
///
/// # Safety
/// `s` must be a pointer returned by [`sc_session_new`] and not already freed.
#[no_mangle]
pub unsafe extern "C" fn sc_session_free(s: *mut ScSession) {
    if !s.is_null() {
        drop(Box::from_raw(s));
    }
}

/// Free a string returned by any `sc_*` function. Safe to call with NULL.
///
/// # Safety
/// `p` must be a pointer returned by one of this library's string functions and
/// not already freed. Do **not** pass a pointer from a different allocator.
#[no_mangle]
pub unsafe extern "C" fn sc_string_free(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}

/// Read an input C string into an owned `String` (lossy on invalid UTF-8). A
/// null pointer becomes the empty string.
///
/// # Safety
/// `p` must be null or a valid NUL-terminated C string.
unsafe fn read_cstr(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    CStr::from_ptr(p).to_string_lossy().into_owned()
}

/// Move a Rust `String` out as a heap C string the caller must free with
/// [`sc_string_free`]. Returns null if the string contains an interior NUL
/// (which the engine's JSON/text output never does).
fn into_cstr(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// `set_cell(a1, raw)` → JSON status string (`{"ok":true}` / error). See
/// [`SpreadsheetSession::set_cell`]. Returns null only on a null `s`.
///
/// # Safety
/// `s` must be a valid session; `a1`/`raw` must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_set_cell(
    s: *mut ScSession,
    a1: *const c_char,
    raw: *const c_char,
) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let a1 = read_cstr(a1);
    let raw = read_cstr(raw);
    into_cstr((*s).inner.set_cell(&a1, &raw))
}

/// `get_value(a1)` → JSON value object. See [`SpreadsheetSession::get_value`].
///
/// # Safety
/// `s` must be a valid session; `a1` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_get_value(s: *mut ScSession, a1: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_value(&read_cstr(a1)))
}

/// `get_raw(a1)` → the typed source string. See
/// [`SpreadsheetSession::get_raw`].
///
/// # Safety
/// `s` must be a valid session; `a1` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_get_raw(s: *mut ScSession, a1: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_raw(&read_cstr(a1)))
}

/// `get_values()` → JSON map of every set cell. See
/// [`SpreadsheetSession::get_values`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_get_values(s: *mut ScSession) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_values())
}

// ── Cell display formats ─────────────────────────────────────────────
// An Excel-style format code per cell decides how its value reads.

/// `set_format(a1, code)` — set a cell's display format (empty `code` clears it).
/// See [`SpreadsheetSession::set_format`].
///
/// # Safety
/// `s` must be a valid session; `a1`/`code` must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_set_format(s: *mut ScSession, a1: *const c_char, code: *const c_char) {
    if s.is_null() {
        return;
    }
    let a1 = read_cstr(a1);
    let code = read_cstr(code);
    (*s).inner.set_format(&a1, &code);
}

/// `get_format(a1)` → the cell's format code, or `""`. See
/// [`SpreadsheetSession::get_format`].
///
/// # Safety
/// `s` must be a valid session; `a1` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_get_format(s: *mut ScSession, a1: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_format(&read_cstr(a1)))
}

/// `get_display(a1)` → the cell's value rendered through its format (the display
/// string). See [`SpreadsheetSession::get_display`].
///
/// # Safety
/// `s` must be a valid session; `a1` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_get_display(s: *mut ScSession, a1: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_display(&read_cstr(a1)))
}

// ── Structural edits: insert / delete rows & columns ─────────────────
// 1-based `at`, `count` lines. The engine relocates cells and rewrites every
// formula's references; the facade keeps its raw echo map in step. No return —
// the host re-reads via get_window / get_raw after the edit.

/// `insert_rows(at, count)`. See [`SpreadsheetSession::insert_rows`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_insert_rows(s: *mut ScSession, at: u32, count: u32) {
    if s.is_null() {
        return;
    }
    (*s).inner.insert_rows(at, count);
}

/// `delete_rows(at, count)`. See [`SpreadsheetSession::delete_rows`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_delete_rows(s: *mut ScSession, at: u32, count: u32) {
    if s.is_null() {
        return;
    }
    (*s).inner.delete_rows(at, count);
}

/// `insert_cols(at, count)`. See [`SpreadsheetSession::insert_cols`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_insert_cols(s: *mut ScSession, at: u32, count: u32) {
    if s.is_null() {
        return;
    }
    (*s).inner.insert_cols(at, count);
}

/// `delete_cols(at, count)`. See [`SpreadsheetSession::delete_cols`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_delete_cols(s: *mut ScSession, at: u32, count: u32) {
    if s.is_null() {
        return;
    }
    (*s).inner.delete_cols(at, count);
}

// ── Viewport primitive (virtualized infinite sheet) ──────────────────
// Integer coordinates (1-based, inclusive) so a native scrolling host can fetch
// just the visible window of an unbounded sheet.

/// `get_window(row0, col0, row1, col1)` → window JSON. See
/// [`SpreadsheetSession::get_window`]. Returns null only on a null `s`.
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_get_window(
    s: *mut ScSession,
    row0: u32,
    col0: u32,
    row1: u32,
    col1: u32,
) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_window(row0, col0, row1, col1))
}

/// `fill(src, dst_start, dst_end)` — replicate the `src` cell across the
/// inclusive rectangle `dst_start`..`dst_end` (drag-fill): relative references
/// shift per target, absolute (`$`) refs pin, the source's format carries along,
/// an empty source clears each target. Malformed addresses are a no-op. See
/// [`SpreadsheetSession::fill`].
///
/// # Safety
/// `s` must be a valid session; the A1 args must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_fill(
    s: *mut ScSession,
    src: *const c_char,
    dst_start: *const c_char,
    dst_end: *const c_char,
) {
    if s.is_null() {
        return;
    }
    let src = read_cstr(src);
    let dst_start = read_cstr(dst_start);
    let dst_end = read_cstr(dst_end);
    (*s).inner.fill(&src, &dst_start, &dst_end);
}

/// `get_display_window(row0, col0, row1, col1)` → display-window JSON (each cell
/// is its value rendered through its format code). See
/// [`SpreadsheetSession::get_display_window`]. Returns null only on a null `s`.
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_get_display_window(
    s: *mut ScSession,
    row0: u32,
    col0: u32,
    row1: u32,
    col1: u32,
) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.get_display_window(row0, col0, row1, col1))
}

/// `used_range()` → data-extent JSON, or the literal `null`. See
/// [`SpreadsheetSession::used_range`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_used_range(s: *mut ScSession) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.used_range())
}

/// `column_letters(index)` → `"A"`/`"AA"`/… for a 1-based column index. See
/// [`SpreadsheetSession::column_letters`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_column_letters(s: *mut ScSession, index: u32) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.column_letters(index))
}

/// `current_revision()` → the per-edit revision clock (a plain integer, not a
/// string). Returns 0 on a null session. See
/// [`SpreadsheetSession::current_revision`].
///
/// # Safety
/// `s` must be a valid session (or null).
#[no_mangle]
pub unsafe extern "C" fn sc_current_revision(s: *mut ScSession) -> u64 {
    if s.is_null() {
        return 0;
    }
    (*s).inner.current_revision()
}

/// `changed_since(since)` → changed-cells JSON. See
/// [`SpreadsheetSession::changed_since`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_changed_since(s: *mut ScSession, since: u64) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.changed_since(since))
}

// ---------------------------------------------------------------------------
// Host-target tests: drive the C ABI from Rust (so they run under `cargo test`
// without a C compiler). A separate test/smoke.c exercises the same calls from
// real C through the built shared library; see build-capi.sh.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// Call a `(session, a1, raw)` setter and return the freed result string.
    unsafe fn set(s: *mut ScSession, a1: &str, raw: &str) -> String {
        let ca = CString::new(a1).unwrap();
        let cr = CString::new(raw).unwrap();
        let out = sc_set_cell(s, ca.as_ptr(), cr.as_ptr());
        let str = CStr::from_ptr(out).to_string_lossy().into_owned();
        sc_string_free(out);
        str
    }

    unsafe fn value(s: *mut ScSession, a1: &str) -> String {
        let ca = CString::new(a1).unwrap();
        let out = sc_get_value(s, ca.as_ptr());
        let str = CStr::from_ptr(out).to_string_lossy().into_owned();
        sc_string_free(out);
        str
    }

    #[test]
    fn c_abi_round_trips_a_formula() {
        unsafe {
            let s = sc_session_new();
            assert_eq!(set(s, "B1", "15"), r#"{"ok":true}"#);
            for (a, v) in [("B2", "8"), ("B3", "12"), ("B4", "4"), ("B5", "7")] {
                set(s, a, v);
            }
            set(s, "B6", "=SUM(B1:B5)");
            assert_eq!(value(s, "B6"), r#"{"kind":"number","value":46.0}"#);
            set(s, "B1", "115");
            assert_eq!(value(s, "B6"), r#"{"kind":"number","value":146.0}"#);
            sc_session_free(s);
        }
    }

    #[test]
    fn c_abi_get_raw_and_error() {
        unsafe {
            let s = sc_session_new();
            set(s, "A1", "=1/0");
            assert_eq!(value(s, "A1"), r##"{"code":"#DIV/0!","kind":"error"}"##);
            let ca = CString::new("A1").unwrap();
            let raw = sc_get_raw(s, ca.as_ptr());
            assert_eq!(CStr::from_ptr(raw).to_string_lossy(), "=1/0");
            sc_string_free(raw);
            sc_session_free(s);
        }
    }

    #[test]
    fn c_abi_fill_replicates_and_shifts() {
        unsafe {
            let s = sc_session_new();
            set(s, "A1", "10");
            set(s, "A2", "20");
            set(s, "B1", "=A1*2"); // 20
            let src = CString::new("B1").unwrap();
            let ds = CString::new("B2").unwrap();
            let de = CString::new("B2").unwrap();
            sc_fill(s, src.as_ptr(), ds.as_ptr(), de.as_ptr());
            assert_eq!(value(s, "B2"), r#"{"kind":"number","value":40.0}"#); // A2*2
            // Null session is a safe no-op (no return value to check).
            sc_fill(ptr::null_mut(), src.as_ptr(), ds.as_ptr(), de.as_ptr());
            sc_session_free(s);
        }
    }

    #[test]
    fn null_handle_returns_null_not_crash() {
        unsafe {
            let ca = CString::new("A1").unwrap();
            assert!(sc_get_value(ptr::null_mut(), ca.as_ptr()).is_null());
            assert!(sc_set_cell(ptr::null_mut(), ca.as_ptr(), ca.as_ptr()).is_null());
            sc_session_free(ptr::null_mut()); // no-op
            sc_string_free(ptr::null_mut()); // no-op
        }
    }

    /// Call an `sc_*` getter returning a char* and return the freed string.
    unsafe fn take(out: *mut c_char) -> String {
        let str = CStr::from_ptr(out).to_string_lossy().into_owned();
        sc_string_free(out);
        str
    }

    #[test]
    fn c_abi_viewport_round_trips() {
        unsafe {
            let s = sc_session_new();
            set(s, "A1", "15");
            set(s, "B1", "3");
            set(s, "C1", "=SUM(A1:B1)"); // 18

            let w = take(sc_get_window(s, 1, 1, 1, 3));
            assert!(w.contains(r#""rows":1"#) && w.contains(r#""cols":3"#), "{w}");
            assert!(w.contains(r#"{"kind":"number","value":18.0}"#), "{w}");

            // Display window: a formatted cell paints its rendered string.
            let cf = CString::new("A1").unwrap();
            let cc = CString::new("#,##0.00").unwrap();
            sc_set_format(s, cf.as_ptr(), cc.as_ptr());
            let dw = take(sc_get_display_window(s, 1, 1, 1, 3));
            assert!(dw.contains(r#""cells""#) && dw.contains("15.00"), "{dw}");
            assert_eq!(
                take(sc_get_display_window(s, 0, 0, 5, 5)),
                r##"{"error":"#REF!"}"##
            );
            assert!(sc_get_display_window(ptr::null_mut(), 1, 1, 1, 1).is_null());

            assert!(take(sc_used_range(s)).contains(r#""maxCol":3"#));
            assert_eq!(take(sc_column_letters(s, 27)), "AA");

            let snap = sc_current_revision(s);
            set(s, "A1", "100");
            let c = take(sc_changed_since(s, snap));
            assert!(c.contains("\"A1\"") && c.contains("\"C1\""), "{c}");

            // Bad window → error object; null handle → null / 0, never a crash.
            assert_eq!(take(sc_get_window(s, 0, 0, 5, 5)), r##"{"error":"#REF!"}"##);
            assert!(sc_get_window(ptr::null_mut(), 1, 1, 1, 1).is_null());
            assert_eq!(sc_current_revision(ptr::null_mut()), 0);

            sc_session_free(s);
        }
    }
}
