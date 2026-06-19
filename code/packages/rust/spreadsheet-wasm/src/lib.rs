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

// ── Cell display formats ─────────────────────────────────────────────
// An Excel-style format code per cell decides how its value reads.

/// `set_format(a1, code)` — set a cell's display format (empty `code` clears).
/// See [`SpreadsheetSession::set_format`].
///
/// # Safety
/// The `(ptr, len)` pairs must describe readable byte ranges (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn set_format(
    a1_ptr: *const u8,
    a1_len: usize,
    code_ptr: *const u8,
    code_len: usize,
) {
    let a1 = read_input(a1_ptr, a1_len);
    let code = read_input(code_ptr, code_len);
    SESSION.with(|s| s.borrow_mut().set_format(&a1, &code));
}

/// `get_format(a1)` → the cell's format code, or `""`. See
/// [`SpreadsheetSession::get_format`].
///
/// # Safety
/// `(ptr, len)` must describe a readable byte range.
#[no_mangle]
pub unsafe extern "C" fn get_format(a1_ptr: *const u8, a1_len: usize) -> *mut u8 {
    let a1 = read_input(a1_ptr, a1_len);
    pack(SESSION.with(|s| s.borrow().get_format(&a1)))
}

/// `get_display(a1)` → the cell's value rendered through its format (the display
/// string). See [`SpreadsheetSession::get_display`].
///
/// # Safety
/// `(ptr, len)` must describe a readable byte range.
#[no_mangle]
pub unsafe extern "C" fn get_display(a1_ptr: *const u8, a1_len: usize) -> *mut u8 {
    let a1 = read_input(a1_ptr, a1_len);
    pack(SESSION.with(|s| s.borrow().get_display(&a1)))
}

// ── Structural edits: insert / delete rows & columns ─────────────────
// 1-based `at`, `count` lines. The engine relocates cells and rewrites formula
// references; the formula echo stays in step. No return — the JS host re-reads
// via get_window / get_raw afterwards.

/// `insert_rows(at, count)`. See [`SpreadsheetSession::insert_rows`].
#[no_mangle]
pub extern "C" fn insert_rows(at: u32, count: u32) {
    SESSION.with(|s| s.borrow_mut().insert_rows(at, count));
}

/// `delete_rows(at, count)`. See [`SpreadsheetSession::delete_rows`].
#[no_mangle]
pub extern "C" fn delete_rows(at: u32, count: u32) {
    SESSION.with(|s| s.borrow_mut().delete_rows(at, count));
}

/// `insert_cols(at, count)`. See [`SpreadsheetSession::insert_cols`].
#[no_mangle]
pub extern "C" fn insert_cols(at: u32, count: u32) {
    SESSION.with(|s| s.borrow_mut().insert_cols(at, count));
}

/// `delete_cols(at, count)`. See [`SpreadsheetSession::delete_cols`].
#[no_mangle]
pub extern "C" fn delete_cols(at: u32, count: u32) {
    SESSION.with(|s| s.borrow_mut().delete_cols(at, count));
}

/// `fill(src, dst_start, dst_end)` — drag-fill: replicate the `src` cell across
/// the inclusive A1 rectangle `dst_start`..`dst_end`, shifting each copy's
/// relative references (absolute `$` refs pin), carrying the source's format,
/// clearing from an empty source. Malformed addresses are a no-op. The JS host
/// re-reads via `get_window` / `get_display_window` / `get_raw` afterwards. See
/// [`SpreadsheetSession::fill`].
///
/// # Safety
/// The three `(ptr, len)` pairs must describe readable byte ranges (see
/// [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn fill(
    src_ptr: *const u8,
    src_len: usize,
    dst_start_ptr: *const u8,
    dst_start_len: usize,
    dst_end_ptr: *const u8,
    dst_end_len: usize,
) {
    let src = read_input(src_ptr, src_len);
    let dst_start = read_input(dst_start_ptr, dst_start_len);
    let dst_end = read_input(dst_end_ptr, dst_end_len);
    SESSION.with(|s| s.borrow_mut().fill(&src, &dst_start, &dst_end));
}

/// `copy(start, end)` — copy the inclusive rectangle `start`..`end` into the
/// clipboard (a whole-block copy that pastes as a unit). The source is left
/// untouched; the buffer survives any number of pastes. See
/// [`SpreadsheetSession::copy`].
///
/// # Safety
/// Both `(ptr, len)` pairs must describe readable byte ranges (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn copy(
    start_ptr: *const u8,
    start_len: usize,
    end_ptr: *const u8,
    end_len: usize,
) {
    let start = read_input(start_ptr, start_len);
    let end = read_input(end_ptr, end_len);
    SESSION.with(|s| s.borrow_mut().copy(&start, &end));
}

/// `cut(start, end)` — like [`copy`] but a one-shot move: the paste that places
/// it clears the source it didn't overwrite and consumes the buffer. See
/// [`SpreadsheetSession::cut`].
///
/// # Safety
/// Both `(ptr, len)` pairs must describe readable byte ranges (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn cut(
    start_ptr: *const u8,
    start_len: usize,
    end_ptr: *const u8,
    end_len: usize,
) {
    let start = read_input(start_ptr, start_len);
    let end = read_input(end_ptr, end_len);
    SESSION.with(|s| s.borrow_mut().cut(&start, &end));
}

/// `paste(dst_start)` — paste the clipboard so its top-left lands at
/// `dst_start`. Returns `1` when applied, `0` for a no-op (empty clipboard,
/// malformed address, or off-grid). Unlike the string-returning exports this
/// returns the flag directly — no pointer to free. See
/// [`SpreadsheetSession::paste`].
///
/// # Safety
/// The `(ptr, len)` pair must describe a readable byte range (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn paste(dst_start_ptr: *const u8, dst_start_len: usize) -> i32 {
    let dst_start = read_input(dst_start_ptr, dst_start_len);
    if SESSION.with(|s| s.borrow_mut().paste(&dst_start)) {
        1
    } else {
        0
    }
}

// ── Save / load (serialize) ──────────────────────────────────────────

/// `serialize()` → a packed JSON document holding the workbook's SOURCE (formula
/// text + typed literals) and per-cell formats — not the computed values, which
/// recompute on load. The host reads it via [`read_output`] and must release it
/// with [`dealloc`], like any packed string. See [`SpreadsheetSession::serialize`].
#[no_mangle]
pub extern "C" fn serialize() -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().serialize()))
}

/// `deserialize(data)` — replace the workbook with a document produced by
/// [`serialize`]. Returns `1` on success, `0` if the data is malformed or an
/// unsupported version (the existing workbook is left untouched on failure).
/// Returns the flag directly — no pointer to free. See
/// [`SpreadsheetSession::deserialize`].
///
/// # Safety
/// The `(ptr, len)` pair must describe a readable byte range (see [`read_input`]).
#[no_mangle]
pub unsafe extern "C" fn deserialize(data_ptr: *const u8, data_len: usize) -> i32 {
    let data = read_input(data_ptr, data_len);
    if SESSION.with(|s| s.borrow_mut().deserialize(&data)) {
        1
    } else {
        0
    }
}

// ── Undo / redo (session history) ────────────────────────────────────
// All four take no arguments and return a flag directly (no string marshalling).

/// `undo()` — revert the most recent edit. Returns `1` if an edit was undone,
/// `0` if there was nothing to undo. The host re-reads the viewport afterwards.
/// See [`SpreadsheetSession::undo`].
#[no_mangle]
pub extern "C" fn undo() -> i32 {
    if SESSION.with(|s| s.borrow_mut().undo()) {
        1
    } else {
        0
    }
}

/// `redo()` — replay the most recently undone edit. Returns `1`/`0`. See
/// [`SpreadsheetSession::redo`].
#[no_mangle]
pub extern "C" fn redo() -> i32 {
    if SESSION.with(|s| s.borrow_mut().redo()) {
        1
    } else {
        0
    }
}

/// `can_undo()` → `1` if there is an edit to undo, else `0`. See
/// [`SpreadsheetSession::can_undo`].
#[no_mangle]
pub extern "C" fn can_undo() -> i32 {
    if SESSION.with(|s| s.borrow().can_undo()) {
        1
    } else {
        0
    }
}

/// `can_redo()` → `1` if there is an undone edit to redo, else `0`. See
/// [`SpreadsheetSession::can_redo`].
#[no_mangle]
pub extern "C" fn can_redo() -> i32 {
    if SESSION.with(|s| s.borrow().can_redo()) {
        1
    } else {
        0
    }
}

// ── Viewport primitive (virtualized infinite sheet) ──────────────────
// These take integer coordinates directly (no pointer marshalling), so a
// scrolling JS host can fetch just the visible window of an unbounded sheet.

/// `get_window(row0, col0, row1, col1)` → window JSON (1-based, inclusive). See
/// [`SpreadsheetSession::get_window`].
#[no_mangle]
pub extern "C" fn get_window(row0: u32, col0: u32, row1: u32, col1: u32) -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().get_window(row0, col0, row1, col1)))
}

/// `get_display_window(row0, col0, row1, col1)` → display-window JSON (each cell
/// rendered through its format code; 1-based, inclusive). See
/// [`SpreadsheetSession::get_display_window`].
#[no_mangle]
pub extern "C" fn get_display_window(row0: u32, col0: u32, row1: u32, col1: u32) -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().get_display_window(row0, col0, row1, col1)))
}

/// `used_range()` → data-extent JSON, or the literal `null`. See
/// [`SpreadsheetSession::used_range`].
#[no_mangle]
pub extern "C" fn used_range() -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().used_range()))
}

/// `column_letters(index)` → `"A"`/`"AA"`/… for a 1-based column index. See
/// [`SpreadsheetSession::column_letters`].
#[no_mangle]
pub extern "C" fn column_letters(index: u32) -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().column_letters(index)))
}

/// `current_revision()` → the per-edit revision clock, returned directly (it's a
/// plain integer, not a packed string). See
/// [`SpreadsheetSession::current_revision`].
#[no_mangle]
pub extern "C" fn current_revision() -> u64 {
    SESSION.with(|s| s.borrow().current_revision())
}

/// `changed_since(since)` → changed-cells JSON. See
/// [`SpreadsheetSession::changed_since`].
#[no_mangle]
pub extern "C" fn changed_since(since: u64) -> *mut u8 {
    pack(SESSION.with(|s| s.borrow().changed_since(since)))
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

    fn set_fmt(a1: &str, code: &str) {
        let (ap, al) = put(a1);
        let (cp, cl) = put(code);
        unsafe { set_format(ap, al, cp, cl) };
        unsafe {
            dealloc(ap, al);
            dealloc(cp, cl);
        }
    }

    fn display(a1: &str) -> String {
        let (ap, al) = put(a1);
        let out = unsafe { get_display(ap, al) };
        unsafe { dealloc(ap, al) };
        take(out)
    }

    fn format(a1: &str) -> String {
        let (ap, al) = put(a1);
        let out = unsafe { get_format(ap, al) };
        unsafe { dealloc(ap, al) };
        take(out)
    }

    #[test]
    fn abi_cell_format_round_trip() {
        reset();
        set("A1", "1234.5");
        assert_eq!(display("A1"), "1234.5"); // General before any format
        set_fmt("A1", "#,##0.00");
        assert_eq!(format("A1"), "#,##0.00");
        assert_eq!(display("A1"), "1,234.50");
        set_fmt("A1", ""); // clear → back to General
        assert_eq!(format("A1"), "");
        assert_eq!(display("A1"), "1234.5");
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
    fn abi_insert_and_delete_rows() {
        reset();
        set("A1", "10");
        set("A2", "20");
        set("A3", "=SUM(A1:A2)");

        insert_rows(1, 1); // a blank row at the top — everything slides down
        assert_eq!(value("A2"), r#"{"kind":"number","value":10.0}"#); // was A1
        assert_eq!(value("A4"), r#"{"kind":"number","value":30.0}"#); // SUM moved
        assert_eq!(raw("A4"), "=SUM(A2:A3)"); // range rewritten

        delete_rows(1, 1); // remove the blank top row again
        assert_eq!(value("A1"), r#"{"kind":"number","value":10.0}"#);
        assert_eq!(value("A3"), r#"{"kind":"number","value":30.0}"#);
        assert_eq!(raw("A3"), "=SUM(A1:A2)");
    }

    #[test]
    fn abi_fill_replicates_and_shifts() {
        reset();
        set("A1", "10");
        set("A2", "20");
        set("B1", "=A1*2"); // 20
        let (sp, sl) = put("B1");
        let (asp, asl) = put("B2");
        let (aep, ael) = put("B2");
        // SAFETY: all three pairs come from `put`/`alloc`.
        unsafe { fill(sp, sl, asp, asl, aep, ael) };
        unsafe {
            dealloc(sp, sl);
            dealloc(asp, asl);
            dealloc(aep, ael);
        }
        assert_eq!(value("B2"), r#"{"kind":"number","value":40.0}"#); // A2*2
        assert_eq!(raw("B2"), "=(A2*2)"); // echoed shifted source
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

    #[test]
    fn abi_viewport_round_trips() {
        reset();
        set("A1", "15");
        set("B1", "3");
        set("C1", "=SUM(A1:B1)"); // 18

        // get_window over A1:C1 — integer coords passed straight through.
        let w = take(get_window(1, 1, 1, 3));
        assert!(w.contains(r#""rows":1"#) && w.contains(r#""cols":3"#), "{w}");
        assert!(w.contains(r#"{"kind":"number","value":18.0}"#), "{w}");

        // Display window: a formatted cell paints its rendered string.
        set_fmt("A1", "#,##0.00");
        let dw = take(get_display_window(1, 1, 1, 3));
        assert!(dw.contains(r#""cells""#) && dw.contains("15.00"), "{dw}");
        assert_eq!(take(get_display_window(0, 0, 5, 5)), r##"{"error":"#REF!"}"##);

        // used_range covers A1..C1.
        let u = take(used_range());
        assert!(u.contains(r#""minRow":1"#) && u.contains(r#""maxCol":3"#), "{u}");

        // column_letters beyond Z.
        assert_eq!(take(column_letters(27)), "AA");

        // revision clock + changed_since diff: editing A1 dirties A1 and its
        // dependent C1, but not B1.
        let snap = current_revision();
        set("A1", "100");
        let c = take(changed_since(snap));
        assert!(c.contains("\"A1\"") && c.contains("\"C1\""), "{c}");
        assert!(!c.contains("\"stale\""), "{c}");

        // A bad window surfaces an error object — never a panic/trap.
        assert_eq!(take(get_window(0, 0, 5, 5)), r##"{"error":"#REF!"}"##);
    }
}
