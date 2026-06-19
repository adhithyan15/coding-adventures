# Changelog

## 0.9.0

**Undo / redo (session history).** New C ABI exports `sc_undo(s)`, `sc_redo(s)`, `sc_can_undo(s)`, `sc_can_redo(s)` — each returns `int` (1/0). `sc_undo`/`sc_redo` return 1 when they changed the document, 0 when there was nothing to do; `sc_can_undo`/`sc_can_redo` report whether the corresponding control should be enabled. Declared in `include/spreadsheet.h`. Null session is safe (every call returns 0). Delegates to spreadsheet-core-wasm 0.9.0's snapshot-based history. 1 round-trip test (edit → undo×2 → redo×2 with a live-recompute check, plus null-safety).

## 0.8.0

**Save / load (serialize).** New C ABI exports `sc_serialize(s) -> char*` (a self-contained JSON document of the workbook's source + formats; free with `sc_string_free`) and `sc_deserialize(s, data) -> int` (1 = loaded, 0 = malformed / unsupported version, in which case the existing workbook is left untouched). Declared in `include/spreadsheet.h`. Null session is safe (serialize → null, deserialize → 0). 1 round-trip test (serialize → load into a fresh session → live recompute; garbage rejected; null-safety).

## 0.7.0

**Clipboard — cut / copy / paste.** New C ABI exports `sc_copy(s, start, end)`, `sc_cut(s, start, end)` (void), and `sc_paste(s, dst_start) -> int` (1 = applied, 0 = no-op for empty clipboard / malformed address / off-grid). Declared in `include/spreadsheet.h`. Null session is a safe no-op (paste returns 0). 1 round-trip test (copy→paste shift, cut→move + source-clear + one-shot, null-safety).

## 0.6.0

**`sc_fill` over the C ABI.** New
`sc_fill(s, src, dst_start, dst_end)` (three A1 C strings, void return),
declared in `include/spreadsheet.h` — drag-fill: replicate the `src` cell across
the inclusive rectangle, relative refs shifting per target, absolute (`$`) refs
pinned, the source's format carried along, an empty source clearing each target;
a malformed address is a no-op. Delegates to `SpreadsheetSession::fill`;
null-session safe. New C ABI round-trip test (formula shift + null-session
no-op).

## 0.5.0

**`sc_get_display_window` over the C ABI.** New
`sc_get_display_window(s, row0, col0, row1, col1)` (→ display-window JSON, freed
with `sc_string_free`), declared in `include/spreadsheet.h`. Like `sc_get_window`
but each cell is its display string (value rendered through its format code;
empty cells `""`) — the one read a native virtualized grid needs per frame.
Delegates to `SpreadsheetSession::get_display_window`; null-session safe. The C
ABI round-trip test now also exercises it (formatted cell + bad-window/null
guards).

## 0.4.0

**Cell display formats over the C ABI.** New `sc_set_format(s, a1, code)` (void),
`sc_get_format(s, a1)` (→ code | `""`), and `sc_get_display(s, a1)` (→ the value
rendered through its format), declared in `include/spreadsheet.h`. Delegate to
`SpreadsheetSession`'s format API; null-session safe.

## 0.3.0

**Insert/delete rows & columns over the C ABI.** New `void`-returning exports
`sc_insert_rows` / `sc_delete_rows` / `sc_insert_cols` / `sc_delete_cols(s, at,
count)` (1-based), declared in `include/spreadsheet.h`. They delegate to
`SpreadsheetSession`'s new structural-edit methods; the host re-reads via
`sc_get_window` / `sc_get_raw` afterwards. Null-session safe (no-op).

## 0.2.0

Viewport C ABI for the virtualized infinite sheet, mirroring
`spreadsheet-core-wasm` 0.2.0 (`include/spreadsheet.h` now pulls in `<stdint.h>`
for the integer widths):

- `sc_get_window(s, row0, col0, row1, col1) -> char*` (window JSON; `{"error":..}`
  on a bad/oversized request).
- `sc_used_range(s) -> char*` (extent JSON or the literal `null`).
- `sc_column_letters(s, index) -> char*` (1-based index → `"A"`/`"AA"`/…).
- `sc_current_revision(s) -> uint64_t` (the per-edit clock; 0 if `s` is NULL).
- `sc_changed_since(s, since) -> char*` (changed-cells JSON).

Each `char*` is freed with `sc_string_free`, per the existing contract; a NULL
session yields NULL / 0 rather than a crash.

## 0.1.0

Initial release — the stable C ABI over the spreadsheet engine, for the native
VisiCalc demos.

- Opaque `ScSession` handle + `sc_session_new`/`sc_session_free`,
  `sc_set_cell`/`sc_get_value`/`sc_get_raw`/`sc_get_values`, and
  `sc_string_free`. NUL-terminated UTF-8 (JSON) results owned by the caller.
- `include/spreadsheet.h` — the committed C header (with `extern "C"` guards).
- crate-type cdylib + staticlib + rlib (dynamic, static, and host-test linking).
- `build-capi.sh` builds the library and runs `test/smoke.c` against it.
- 3 host-target tests + the C smoke test; zero clippy warnings.
- `verify-native.sh` compiles + runs the same checks from every native language
  the cross-backend demos use, against the built library, proving the engine
  computes identically from each runtime:
  C (`smoke.c`, Qt/C++), Swift (`smoke.swift`, SwiftUI), Dart (`smoke.dart`,
  Flutter via `dart:ffi`), .NET (`test/dotnet/`, XAML via P/Invoke), and Kotlin
  (`smoke.kt`, Compose/Android via Java FFM — no hand-written JNI). All five
  pass on macOS.

The native sibling of `spreadsheet-wasm` (WASM ABI) over the same
`spreadsheet-core-wasm` facade. Next: per-platform bindings (Swift, then JNI /
dart:ffi / P/Invoke) and wiring each native demo.
