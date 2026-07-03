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
use std::os::raw::{c_char, c_int};
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

/// `copy(start, end)` — copy the inclusive rectangle `start`..`end` into the
/// clipboard (a whole-block copy that pastes as a unit). The source is left
/// untouched and the buffer survives any number of pastes. Malformed addresses
/// or an oversized rectangle are a no-op. See [`SpreadsheetSession::copy`].
///
/// # Safety
/// `s` must be a valid session; the A1 args must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_copy(s: *mut ScSession, start: *const c_char, end: *const c_char) {
    if s.is_null() {
        return;
    }
    let start = read_cstr(start);
    let end = read_cstr(end);
    (*s).inner.copy(&start, &end);
}

/// `cut(start, end)` — like [`sc_copy`] but a one-shot move: the paste that
/// places it clears the source cells it didn't overwrite and consumes the
/// buffer. The source is not cleared until paste. See
/// [`SpreadsheetSession::cut`].
///
/// # Safety
/// `s` must be a valid session; the A1 args must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_cut(s: *mut ScSession, start: *const c_char, end: *const c_char) {
    if s.is_null() {
        return;
    }
    let start = read_cstr(start);
    let end = read_cstr(end);
    (*s).inner.cut(&start, &end);
}

/// `paste(dst_start)` — paste the clipboard so its top-left lands at
/// `dst_start`. Returns `1` when a paste was applied, `0` for a no-op (empty
/// clipboard, malformed address, or a destination past the grid edge). The
/// block's references shift by the destination's offset from the source anchor;
/// content, format, and the source echo ride along. See
/// [`SpreadsheetSession::paste`].
///
/// # Safety
/// `s` must be a valid session; `dst_start` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_paste(s: *mut ScSession, dst_start: *const c_char) -> c_int {
    if s.is_null() {
        return 0;
    }
    let dst_start = read_cstr(dst_start);
    if (*s).inner.paste(&dst_start) {
        1
    } else {
        0
    }
}

/// `sort_range(start, end, key_col, ascending)` — reorder the rows of the
/// inclusive rectangle `start`..`end` by the computed values in `key_col` (a
/// 1-based absolute column index inside the rectangle). `ascending` is a flag
/// (`0` = descending, anything else ascending). Each row moves as a record;
/// moved formulas have their relative references shifted with their row, and
/// formats ride along. Returns `1` when a valid sort is applied (or the range was
/// already sorted), `0` for a malformed address, an out-of-range `key_col`, an
/// empty/single-row range, or an oversized rectangle. See
/// [`SpreadsheetSession::sort_range`].
///
/// # Safety
/// `s` must be a valid session; the A1 args must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_sort_range(
    s: *mut ScSession,
    start: *const c_char,
    end: *const c_char,
    key_col: u32,
    ascending: c_int,
) -> c_int {
    if s.is_null() {
        return 0;
    }
    let start = read_cstr(start);
    let end = read_cstr(end);
    if (*s).inner.sort_range(&start, &end, key_col, ascending != 0) {
        1
    } else {
        0
    }
}

/// `find_all(query, in_formulas, ascending_case_flags)` — locate cells whose text
/// contains `query`. `in_formulas` (flag: non-zero searches each cell's source,
/// 0 its computed display value) and `match_case` (flag: 0 folds ASCII case).
/// Returns a heap `char*` JSON object `{"matches":["A1",…]}` (A1 addresses in
/// (row,col) order); free it with [`sc_string_free`]. An empty query → empty list.
///
/// # Safety
/// `s` must be a valid session; `query` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_find_all(
    s: *mut ScSession,
    query: *const c_char,
    in_formulas: c_int,
    match_case: c_int,
) -> *mut c_char {
    if s.is_null() {
        return into_cstr(String::from("{\"matches\":[]}"));
    }
    let query = read_cstr(query);
    into_cstr((*s).inner.find_all(&query, in_formulas != 0, match_case != 0))
}

/// `replace_all(query, replacement, match_case)` — replace `query` with
/// `replacement` in the source of every matching cell (the engine rewrites +
/// recomputes; the facade keeps its source echo in step). `match_case` is a flag
/// (0 folds ASCII case). Returns the count of cells changed; an empty query is a
/// no-op returning 0. The host re-reads via sc_get_window / sc_get_raw afterwards.
///
/// # Safety
/// `s` must be a valid session; the string args must be null or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn sc_replace_all(
    s: *mut ScSession,
    query: *const c_char,
    replacement: *const c_char,
    match_case: c_int,
) -> c_int {
    if s.is_null() {
        return 0;
    }
    let query = read_cstr(query);
    let replacement = read_cstr(replacement);
    (*s).inner.replace_all(&query, &replacement, match_case != 0) as c_int
}

// ── Multi-sheet workbook ─────────────────────────────────────────────
//
// A session has one or more named sheets; bare-A1 cell ops address the ACTIVE
// sheet, and a formula may reference another (`=Summary!A1`). These manage the
// sheet set + the active sheet; the host re-reads cells/raw afterwards.

/// `sheet_names()` → a heap `char*` JSON object
/// `{"sheets":["Sheet1",…],"active":0}` (names in tab order + the active 0-based
/// index). Free it with [`sc_string_free`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_sheet_names(s: *mut ScSession) -> *mut c_char {
    if s.is_null() {
        return into_cstr(String::from("{\"sheets\":[],\"active\":0}"));
    }
    into_cstr((*s).inner.sheet_names())
}

/// The active sheet's 0-based index (0 if `s` is null).
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_active_sheet(s: *mut ScSession) -> u32 {
    if s.is_null() {
        return 0;
    }
    (*s).inner.active_sheet()
}

/// Switch the active sheet by 0-based `index`. Returns 1 on success, 0 for an
/// out-of-range index (or null session).
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_set_active_sheet(s: *mut ScSession, index: u32) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.set_active_sheet(index) as c_int
}

/// Add a new sheet named `name` and make it active. Returns 1 on success, 0 for
/// an empty/duplicate name (or null session/string).
///
/// # Safety
/// `s` must be a valid session; `name` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_add_sheet(s: *mut ScSession, name: *const c_char) -> c_int {
    if s.is_null() {
        return 0;
    }
    let name = read_cstr(name);
    (*s).inner.add_sheet(&name) as c_int
}

/// Rename the sheet at `index` to `new_name` (rewrites referencing formulas'
/// qualifiers). Returns 1 on success, 0 for a bad index / empty / duplicate name.
///
/// # Safety
/// `s` must be a valid session; `new_name` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_rename_sheet(
    s: *mut ScSession,
    index: u32,
    new_name: *const c_char,
) -> c_int {
    if s.is_null() {
        return 0;
    }
    let new_name = read_cstr(new_name);
    (*s).inner.rename_sheet(index, &new_name) as c_int
}

/// Delete the sheet at `index` (inbound refs → `#REF!`). Returns 1 on success, 0
/// for a bad index or an attempt to delete the last sheet.
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_delete_sheet(s: *mut ScSession, index: u32) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.delete_sheet(index) as c_int
}

/// Move the sheet at `index` to 0-based `to_index` (clamped). Returns 1 on
/// success, 0 for a bad index.
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_move_sheet(s: *mut ScSession, index: u32, to_index: u32) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.move_sheet(index, to_index) as c_int
}

/// `serialize()` → a self-contained JSON document capturing the workbook's
/// source (formula text + typed literals) and per-cell formats — everything
/// needed to reconstruct the sheet, but not the computed values (those recompute
/// on load, so the file is small and can never disagree with itself). Hand the
/// returned string to `sc_deserialize` to restore it. The caller owns the string
/// and must release it with `sc_free_string`. See [`SpreadsheetSession::serialize`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_serialize(s: *mut ScSession) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    into_cstr((*s).inner.serialize())
}

/// `deserialize(data)` — replace the workbook with the contents of a document
/// produced by `sc_serialize`. Returns `1` on success, `0` if `data` is malformed
/// or carries an unsupported version (in which case the existing workbook is left
/// untouched — the engine validates before it mutates). Formulas reload live and
/// recompute. See [`SpreadsheetSession::deserialize`].
///
/// # Safety
/// `s` must be a valid session; `data` must be null or a valid C string.
#[no_mangle]
pub unsafe extern "C" fn sc_deserialize(s: *mut ScSession, data: *const c_char) -> c_int {
    if s.is_null() {
        return 0;
    }
    let data = read_cstr(data);
    if (*s).inner.deserialize(&data) {
        1
    } else {
        0
    }
}

/// `undo()` — revert the most recent edit, restoring the document to its state
/// before that edit. Returns `1` if an edit was undone, `0` if there was nothing
/// to undo (or `s` is null). The host re-reads via `sc_get_window` /
/// `sc_get_display_window` / `sc_get_raw` afterwards. See
/// [`SpreadsheetSession::undo`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_undo(s: *mut ScSession) -> c_int {
    if s.is_null() {
        return 0;
    }
    if (*s).inner.undo() {
        1
    } else {
        0
    }
}

/// `redo()` — replay the most recently undone edit. Returns `1` if an edit was
/// redone, `0` if there was nothing to redo (or `s` is null). See
/// [`SpreadsheetSession::redo`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_redo(s: *mut ScSession) -> c_int {
    if s.is_null() {
        return 0;
    }
    if (*s).inner.redo() {
        1
    } else {
        0
    }
}

/// `can_undo()` → `1` if there is an edit to undo, else `0` (and `0` on a null
/// `s`). Lets a host enable/disable an Undo control. See
/// [`SpreadsheetSession::can_undo`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_can_undo(s: *mut ScSession) -> c_int {
    if s.is_null() {
        return 0;
    }
    if (*s).inner.can_undo() {
        1
    } else {
        0
    }
}

/// `can_redo()` → `1` if there is an undone edit to redo, else `0` (and `0` on a
/// null `s`). See [`SpreadsheetSession::can_redo`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_can_redo(s: *mut ScSession) -> c_int {
    if s.is_null() {
        return 0;
    }
    if (*s).inner.can_redo() {
        1
    } else {
        0
    }
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

// ── Column widths & row heights ──────────────────────────────────────────
// Per-column / per-row sizes on the active sheet. A `double` of `0.0` means
// "no custom size — use the host default" (a valid size is always `> 0`).

/// `column_width(col)` → the active sheet's width for a 1-based `col`, or `0.0`
/// if the column has no custom width (the host uses its default). `0.0` on a null
/// session. See [`SpreadsheetSession::column_width`].
///
/// # Safety
/// `s` must be a valid session (or null).
#[no_mangle]
pub unsafe extern "C" fn sc_column_width(s: *mut ScSession, col: u32) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    (*s).inner.column_width(col)
}

/// `row_height(row)` → the active sheet's height for a 1-based `row`, or `0.0`
/// if unset. The row analogue of [`sc_column_width`].
///
/// # Safety
/// `s` must be a valid session (or null).
#[no_mangle]
pub unsafe extern "C" fn sc_row_height(s: *mut ScSession, row: u32) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    (*s).inner.row_height(row)
}

/// `set_column_width(col, width)` → 1 if it changed, 0 if rejected (non-finite /
/// `≤ 0` width, `col == 0`) or a null session. See
/// [`SpreadsheetSession::set_column_width`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_set_column_width(s: *mut ScSession, col: u32, width: f64) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.set_column_width(col, width) as c_int
}

/// `set_row_height(row, height)` → 1 if it changed, 0 if rejected. The row
/// analogue of [`sc_set_column_width`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_set_row_height(s: *mut ScSession, row: u32, height: f64) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.set_row_height(row, height) as c_int
}

/// `clear_column_width(col)` → 1 if a width was removed (back to the host
/// default), 0 otherwise. See [`SpreadsheetSession::clear_column_width`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_clear_column_width(s: *mut ScSession, col: u32) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.clear_column_width(col) as c_int
}

/// `clear_row_height(row)` → 1 if a height was removed, 0 otherwise.
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_clear_row_height(s: *mut ScSession, row: u32) -> c_int {
    if s.is_null() {
        return 0;
    }
    (*s).inner.clear_row_height(row) as c_int
}

/// `column_widths(col0, col1)` → a heap `char*` JSON array of the customized
/// column widths in `[col0, col1]` on the active sheet, e.g.
/// `[{"col":3,"w":140.0}]` (sorted by column). Free with [`sc_string_free`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_column_widths(s: *mut ScSession, col0: u32, col1: u32) -> *mut c_char {
    if s.is_null() {
        return into_cstr(String::from("[]"));
    }
    into_cstr((*s).inner.column_widths(col0, col1))
}

/// `row_heights(row0, row1)` → a heap `char*` JSON array of the customized row
/// heights in `[row0, row1]`, e.g. `[{"row":2,"h":40.0}]`. Free with
/// [`sc_string_free`].
///
/// # Safety
/// `s` must be a valid session.
#[no_mangle]
pub unsafe extern "C" fn sc_row_heights(s: *mut ScSession, row0: u32, row1: u32) -> *mut c_char {
    if s.is_null() {
        return into_cstr(String::from("[]"));
    }
    into_cstr((*s).inner.row_heights(row0, row1))
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
    fn c_abi_copy_cut_paste() {
        unsafe {
            let s = sc_session_new();
            set(s, "B1", "5");
            set(s, "C1", "=B1*2"); // 10
            // Copy the 1×2 block B1:C1 and paste at B2.
            let bs = CString::new("B1").unwrap();
            let ce = CString::new("C1").unwrap();
            sc_copy(s, bs.as_ptr(), ce.as_ptr());
            let b2 = CString::new("B2").unwrap();
            assert_eq!(sc_paste(s, b2.as_ptr()), 1); // applied
            assert_eq!(value(s, "C2"), r#"{"kind":"number","value":10.0}"#); // B2*2

            // Cut A-col cell and move it; paste returns 1, a second paste 0.
            set(s, "A1", "7");
            let a1 = CString::new("A1").unwrap();
            sc_cut(s, a1.as_ptr(), a1.as_ptr());
            let e1 = CString::new("E1").unwrap();
            assert_eq!(sc_paste(s, e1.as_ptr()), 1);
            assert_eq!(value(s, "E1"), r#"{"kind":"number","value":7.0}"#);
            assert_eq!(value(s, "A1"), r#"{"kind":"empty"}"#); // source cleared
            let g1 = CString::new("G1").unwrap();
            assert_eq!(sc_paste(s, g1.as_ptr()), 0); // buffer consumed

            // Null session: copy/cut no-op, paste returns 0 (never a crash).
            sc_copy(ptr::null_mut(), bs.as_ptr(), ce.as_ptr());
            assert_eq!(sc_paste(ptr::null_mut(), b2.as_ptr()), 0);
            sc_session_free(s);
        }
    }

    #[test]
    fn c_abi_serialize_round_trips() {
        unsafe {
            let s = sc_session_new();
            set(s, "A1", "12");
            set(s, "B1", "=A1*3"); // 36
            // Serialize, then load into a fresh session through the C ABI.
            let saved_ptr = sc_serialize(s);
            let saved = CStr::from_ptr(saved_ptr).to_string_lossy().into_owned();
            sc_string_free(saved_ptr);

            let t = sc_session_new();
            let cdata = CString::new(saved).unwrap();
            assert_eq!(sc_deserialize(t, cdata.as_ptr()), 1);
            assert_eq!(value(t, "A1"), r#"{"kind":"number","value":12.0}"#);
            assert_eq!(value(t, "B1"), r#"{"kind":"number","value":36.0}"#);
            // The formula is live, not frozen: editing A1 recomputes B1.
            set(t, "A1", "100");
            assert_eq!(value(t, "B1"), r#"{"kind":"number","value":300.0}"#);

            // Malformed input is rejected (0) and leaves the workbook untouched.
            let bad = CString::new("nonsense").unwrap();
            assert_eq!(sc_deserialize(t, bad.as_ptr()), 0);
            assert_eq!(value(t, "A1"), r#"{"kind":"number","value":100.0}"#);

            // Null session: serialize → null, deserialize → 0.
            assert!(sc_serialize(ptr::null_mut()).is_null());
            assert_eq!(sc_deserialize(ptr::null_mut(), cdata.as_ptr()), 0);
            sc_session_free(s);
            sc_session_free(t);
        }
    }

    #[test]
    fn c_abi_undo_redo() {
        unsafe {
            let s = sc_session_new();
            // Fresh session: nothing to undo or redo.
            assert_eq!(sc_can_undo(s), 0);
            assert_eq!(sc_can_redo(s), 0);

            set(s, "A1", "1");
            set(s, "B1", "=A1*10"); // 10
            assert_eq!(value(s, "B1"), r#"{"kind":"number","value":10.0}"#);
            assert_eq!(sc_can_undo(s), 1);

            // Undo the formula, then the literal.
            assert_eq!(sc_undo(s), 1);
            assert_eq!(value(s, "B1"), r#"{"kind":"empty"}"#);
            assert_eq!(sc_undo(s), 1);
            assert_eq!(value(s, "A1"), r#"{"kind":"empty"}"#);
            assert_eq!(sc_can_undo(s), 0);

            // Redo both: B1 recomputes live (10).
            assert_eq!(sc_redo(s), 1);
            assert_eq!(value(s, "A1"), r#"{"kind":"number","value":1.0}"#);
            assert_eq!(sc_redo(s), 1);
            assert_eq!(value(s, "B1"), r#"{"kind":"number","value":10.0}"#);
            assert_eq!(sc_can_redo(s), 0);
            assert_eq!(sc_redo(s), 0); // nothing left to redo

            // Null session: every call is a safe 0.
            assert_eq!(sc_undo(ptr::null_mut()), 0);
            assert_eq!(sc_redo(ptr::null_mut()), 0);
            assert_eq!(sc_can_undo(ptr::null_mut()), 0);
            assert_eq!(sc_can_redo(ptr::null_mut()), 0);
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
