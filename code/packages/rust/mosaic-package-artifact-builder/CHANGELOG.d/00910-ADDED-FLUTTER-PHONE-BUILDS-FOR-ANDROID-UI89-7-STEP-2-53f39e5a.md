### Added — Flutter phone builds for Android (UI89 §7, step 2)

- **The runtime directory.** `--runtime-library <dir>` for the Flutter
  backend takes a phone runtime: `android/<abi>/libmosaic_app.so`.
  - It is read strictly and never recursively. The top level may hold only
    `android/`, a real directory. `android/` goes through Compose's
    `android_jni_libs`, and each library through
    `read_regular_file_without_links`, and must be ELF.
  - `ios/` is refused until step 3, and so is any other entry or Compose's
    bare jniLibs.
  - Each library is installed at `runtime/android/<abi>/libmosaic_app.so`.
  - `runtime/` is cleared before any Flutter runtime install, desktop or
    phone.
- **`hook/build.dart` for phones** picks the library for the ABI being
  built. It refuses a non-Android target, an architecture outside the
  table, a missing ABI, a non-ELF file, or the wrong ELF machine. It never
  falls back to another ABI's library. The CodeAsset is unchanged.
- **State.** A phone build pins `path_provider` 2.1.6, and its `main()`
  sets `mosaicStateRoot` from `getApplicationSupportDirectory()` before
  `runApp`. A desktop build's `pubspec.yaml` and `main.dart` are unchanged.
- **The README** gives the `flutter create --platforms=android --org <org>
  --project-name <name> .` command, derived from the manifest's bundle
  identifier, and the `allowBackup="false"` edit. When any part of the
  identifier is not a Dart package name or a valid Android package
  segment, it gives no command and says why.
