# Changelog

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
