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

/* File open / save — bytes in, bytes out (.xlsx/.xls/.csv/.tsv/.json). */
int      sc_load_xlsx(ScSession *s, const uint8_t *bytes, size_t len); /* 1/0 */
uint8_t *sc_save_xlsx(ScSession *s, size_t *out_len);                  /* free w/ sc_bytes_free */
void     sc_bytes_free(uint8_t *ptr, size_t len);
/* …and sc_load_xls/csv/tsv/json + sc_save_xls/csv/tsv/json, same shape. */
```

**Memory contract:** every `char *` result is a heap-allocated, NUL-terminated
UTF-8 string the caller must free with `sc_string_free()` (not the C `free()` —
different allocator). A NULL return signals an error. The value-JSON shape
matches the TypeScript and WASM engines exactly, so every frontend parses
identical output.

**File open / save:** `sc_load_<fmt>(s, bytes, len)` opens a real spreadsheet
file the user picked (returns `1`, or `0` if it isn't a readable file of that
format — the open document is left untouched on failure); `sc_save_<fmt>(s,
&out_len)` returns the current document serialized to that format's bytes (freed
with `sc_bytes_free(ptr, out_len)`). File bytes are **binary** and may contain
NUL, so they cross as an explicit `(ptr, len)` pair, never a C string. `.xlsx`
keeps live formulas; `.xls`/CSV/TSV/JSON are lower-fidelity (values only).

## Build & test

```bash
bash build-capi.sh      # builds the .dylib/.so and runs test/smoke.c against it
bash verify-native.sh   # runs the smoke from EVERY native language (see below)
```

- `cargo test -p spreadsheet-capi` — host tests driving the C ABI from Rust.
- `verify-native.sh` compiles + runs the same checks from every native language
  the cross-backend demos use, against the built library — proving the engine
  computes identically (SUM = 46, AVERAGE = 9.2, `#DIV/0!`, recalc = 146) from
  each runtime:

  | Language | path it represents | how it binds |
  |---|---|---|
  | C (`test/smoke.c`) | Qt / C++ | the header directly |
  | Swift (`test/smoke.swift`) | SwiftUI | C header via the clang importer |
  | Dart (`test/smoke.dart`) | Flutter | `dart:ffi`, opens the shared lib |
  | .NET (`test/dotnet/`) | XAML / WinUI | P/Invoke (`DllImport`) |
  | Kotlin (`test/smoke.kt`) | Compose / Android | Java FFM (no hand-written JNI) |

  Each is skipped (with a note) if its toolchain isn't installed; all five pass
  on macOS with the standard toolchains.
