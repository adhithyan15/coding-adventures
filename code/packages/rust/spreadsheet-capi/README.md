# spreadsheet-capi

A **stable C ABI** over the Rust spreadsheet engine, so the *native* VisiCalc
demos can drive the same engine the web demos run as WASM. It is the native
sibling of `spreadsheet-wasm`: same facade (`spreadsheet-core-wasm`) below,
different boundary on top (NUL-terminated C strings + an opaque handle instead
of WASM linear memory).

```text
  Qt/C++ · SwiftUI · Compose/Android · Flutter · XAML
        │  link libspreadsheet_capi + include spreadsheet.h
        ▼
  spreadsheet-capi   ← this crate (extern "C")
        │
  spreadsheet-core-wasm  →  spreadsheet-core (cells, graph, recalc, formulas)
```

| Platform        | how it binds |
|-----------------|--------------|
| Qt / C++        | include `spreadsheet.h` |
| SwiftUI / Swift | C interop via a module map |
| Compose/Android | JNI → these C functions |
| Flutter / Dart  | `dart:ffi` |
| XAML / .NET     | P/Invoke |

## API (`include/spreadsheet.h`)

```c
ScSession *sc_session_new(void);
void       sc_session_free(ScSession *s);
char *sc_set_cell(ScSession *s, const char *a1, const char *raw); /* {"ok":...} */
char *sc_get_value(ScSession *s, const char *a1);                 /* value JSON  */
char *sc_get_raw(ScSession *s, const char *a1);                   /* typed source */
char *sc_get_values(ScSession *s);                                /* {a1: value} */
void  sc_string_free(char *p);
```

**Memory contract:** every `char *` result is a heap-allocated, NUL-terminated
UTF-8 string the caller must free with `sc_string_free()` (not the C `free()` —
different allocator). A NULL return signals an error. The value-JSON shape
matches the TypeScript and WASM engines exactly, so every frontend parses
identical output.

## Build & test

```bash
bash build-capi.sh      # builds the .dylib/.so and runs test/smoke.c against it
bash verify-native.sh   # also runs the Swift + Dart/FFI smokes (real native langs)
```

- `cargo test -p spreadsheet-capi` — host tests driving the C ABI from Rust.
- `test/smoke.c` — a real **C** program linked against the built library.
- `test/smoke.swift` — the same checks from **Swift** (the SwiftUI path), via
  the C header (clang importer).
- `test/smoke.dart` — the same checks from **Dart** through `dart:ffi` (the
  Flutter path), opening the shared library at runtime.

All three assert SUM = 46, AVERAGE = 9.2, `#DIV/0!`, and incremental recalc
(146) — proving the engine computes identically when driven from C, Swift, and
Dart. The remaining native frontends (Kotlin/JNI, .NET/P-Invoke, C++/Qt) bind
the very same C ABI.
