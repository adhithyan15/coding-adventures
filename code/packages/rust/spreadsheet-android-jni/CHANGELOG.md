# Changelog — spreadsheet-android-jni

## [0.2.0] — 2026-07-07

### Added

**File open / save over JNI — a Java `byte[]` in, a `byte[]` out (SSIO PR7).**
The Android host can now open a real spreadsheet file the user picked and save
the current document as one, over the one engine — the Android arm of the same
open/save story as the WASM (`spreadsheet-wasm`) and C-ABI (`spreadsheet-capi`)
facades.

- `nativeLoadXlsx` / `nativeLoadXls` / `nativeLoadCsv` / `nativeLoadTsv` /
  `nativeLoadJson` `(handle: jlong, data: byte[]) -> boolean` — read a file's
  bytes and replace the current document; `true` = opened, `false` = not a
  readable file of that format (or a dead handle). A failed open leaves the
  document untouched.
- `nativeSaveXlsx` / `nativeSaveXls` / `nativeSaveCsv` / `nativeSaveTsv` /
  `nativeSaveJson` `(handle: jlong) -> byte[]` — serialize the current document
  to that format's bytes.
- Wraps `SpreadsheetSession`'s `*_bytes` methods; the `byte[]` marshalling uses
  `jni-bridge` 0.2.0's new `jni_get_byte_array` / `jni_new_byte_array_from` (a
  `byte[]` carries raw file bytes intact — an `.xlsx` is a ZIP, an `.xls` an OLE2
  file — with no UTF-8 / NUL surprises).
- `.xlsx` keeps live formulas; `.xls` / CSV / TSV / JSON are lower-fidelity
  (values only) per `spreadsheet-io`. Verified: host build + Android
  (`aarch64-linux-android`) compile; the byte marshalling is covered by
  `jni-bridge`'s mock-JNIEnv tests.

## [0.1.0]

### Added

Initial release. JNI bridge from the `com.example.visicalc` Android host to the
shared Rust spreadsheet engine: `nativeNewSession` / `nativeFree`,
`nativeSetCell`, `nativeGetDisplay` / `nativeGetRaw`, `nativeGetDisplayWindow`,
`nativeColumnLetters` — all `Java_<package>_<Class>_<method>` exports over the
zero-dependency `jni-bridge` crate.
