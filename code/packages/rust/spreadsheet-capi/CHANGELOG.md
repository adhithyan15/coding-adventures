# Changelog

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

The native sibling of `spreadsheet-wasm` (WASM ABI) over the same
`spreadsheet-core-wasm` facade. Next: per-platform bindings (Swift, then JNI /
dart:ffi / P/Invoke) and wiring each native demo.
