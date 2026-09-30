### Added — the Android app carries its Rust runtime (UI89 step 5, §3.6)

- A Compose `--runtime-library` that is a directory of per-ABI libraries
  (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`, each holding
  `libmosaic_app.so`) is installed into `android/src/main/jniLibs`. It is
  checked strictly: only those ABI directories, each a real directory with
  exactly one regular `libmosaic_app.so`, no links followed, at least one
  ABI. The desktop project bundles nothing from it.
- `code/scripts/build-mosaic-android-libs.sh` builds such a directory with
  `cargo ndk` at API level 26.

