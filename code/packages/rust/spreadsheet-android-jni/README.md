# spreadsheet-android-jni

The **JNI bridge** from the `com.example.visicalc` Android host to the shared
Rust spreadsheet engine — the Android arm of the cross-platform VisiCalc, over
the same `spreadsheet-core-wasm::SpreadsheetSession` the C-ABI and WASM facades
wrap.

## Why JNI (and not the JVM FFM API)

The Compose **Desktop** demo loads the engine through the Foreign Function &
Memory API (JDK 21+). Android's ART runtime has no FFM, so a native library
there is reached the classic way: `System.loadLibrary` + `native` methods whose
symbols are `Java_<package>_<Class>_<method>`. We build those `extern "C"`
exports directly on the **zero-dependency** [`jni-bridge`](../jni-bridge) crate
(no `jni` / `jni-sys` / `bindgen`), cross-compile to a per-ABI `.so`, and drop it
in the app's `jniLibs/`.

```text
  Android host (Kotlin/Compose)
        │  System.loadLibrary("spreadsheet_android_jni")
        │  external fun nativeSetCell(handle, "B6", "=SUM(B1:B5)")
        ▼
  spreadsheet-android-jni   ← this crate (Java_… exports, over jni-bridge)
        ▼
  spreadsheet-core-wasm     ← SpreadsheetSession (the shared facade)
        ▼
  spreadsheet-core          ← cells, dependency graph, recalc, formulas
```

## API

A session lives on the native heap as a boxed `SpreadsheetSession`; Java holds it
as an opaque `long` handle and must call `nativeFree` when done. All calls are
expected on a single (UI) thread — the handle is not synchronised.

- **Session:** `nativeNewSession() -> long`, `nativeFree(handle)`.
- **Cells:** `nativeSetCell(handle, a1, raw) -> String` (JSON status),
  `nativeGetDisplay` / `nativeGetRaw(handle, a1) -> String`,
  `nativeGetDisplayWindow(handle, r0, c0, r1, c1) -> String` (grid JSON),
  `nativeColumnLetters(handle, index) -> String`.
- **File open / save (bytes in, bytes out):**
  `nativeLoadXlsx` / `nativeLoadXls` / `nativeLoadCsv` / `nativeLoadTsv` /
  `nativeLoadJson` `(handle, byte[]) -> boolean` open a real file the user
  picked (`true` = opened, `false` = unreadable / dead handle; the document is
  left untouched on failure); `nativeSaveXlsx` / … / `nativeSaveJson`
  `(handle) -> byte[]` return the current document serialized to that format. A
  `byte[]` carries the raw file bytes intact — an `.xlsx` is a ZIP, an `.xls` an
  OLE2 file. `.xlsx` keeps live formulas; `.xls`/CSV/TSV/JSON are lower-fidelity
  (values only) per [`spreadsheet-io`](../spreadsheet-io).

## Build & test

This crate builds and tests on the **host** with `cargo test` (no JVM needed for
compilation). Cross-compile for Android with the installed NDK targets, e.g.
`cargo build -p spreadsheet-android-jni --target aarch64-linux-android`. The
`byte[]` marshalling is covered by [`jni-bridge`](../jni-bridge)'s mock-JNIEnv
tests (a real JNIEnv comes only from a running JVM).
