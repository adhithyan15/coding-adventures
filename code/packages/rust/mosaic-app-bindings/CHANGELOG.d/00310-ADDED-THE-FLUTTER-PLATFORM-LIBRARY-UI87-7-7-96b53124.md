### Added — the Flutter platform library (UI87 §7.7)

- `flutter_platform_effects()` returns `FlutterPlatformEffects { library, core }`:
  `templates/flutter/mosaic_platform_effects.dart` (the `package:file_selector`
  dialogs and `installMosaicPlatformEffects(host, appKinds:)`, re-exporting the
  core) and `templates/flutter/mosaic_platform_effects_core.dart` (plain Dart:
  the contract, the file I/O and the router). Two files because Dart has no
  conditional compilation and the dialogs need the Flutter engine.
- The core answers the Compose library's contract exactly: the same kinds,
  limits (50 MiB open, bounded while reading; 16 MiB save, checked on the
  encoded length first), MIME table in the same order, plain-name rule (by
  code point, lone surrogates refused, Unicode categories through `\p{…}`
  regular expressions, the same default-ignorable ranges and blanks),
  executable list and every failure message; the base64 alphabet is checked
  first because Dart's decoder also takes the URL-safe alphabet and `%3D`.
- Routing (UI87 §7.2) with an `Expando`-recognised router, so a second install
  changes nothing. Each standard `Await` is copied, deferred, and its dialog
  started from a microtask after the settle; the file is read or written in a
  background isolate. Every path after deferral ends in an answer; a disposed
  host swallows it. Android and iOS fail each request with
  "`<kind>` is not available on this platform yet".
- The router holds the `MosaicHost` it was installed on (`MosaicHostEffects`),
  so a late answer after a retried start meets the disposed runtime and is
  dropped. `MosaicHost` gains an `effectHandler` getter for it.
- Save on POSIX through libc (dart:ffi), as Qt and SwiftUI do it:
  `open(O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600)` on a
  temporary beside the target, then write, `fchmod`, `fsync` and `close`
  through that descriptor, and `rename` into place; `unlink` only on failure.
  A link or file planted at the temporary's name fails the save. The mode is
  the replaced regular file's rwx bits only when this user owns it (Linux
  `statx`, Apple `lstat`); otherwise 0600, or `& 0755` where the owner cannot
  be read. The `open(2)` flags come from per-ABI tables (Linux x64 and arm64,
  macOS). Windows: `CreateFileW(CREATE_NEW, FILE_FLAG_OPEN_REPARSE_POINT)`, `WriteFile`, `FlushFileBuffers`,
  `CloseHandle` on that handle, then
  `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`.
- Open on POSIX: after the path check, one `open(O_RDONLY | O_NONBLOCK |
  O_NOFOLLOW | O_CLOEXEC)` typed with `fstat` and read through that
  descriptor, so a FIFO swapped in cannot block the read; where the type
  cannot be had (no `statx`, or refused), the read fails closed.
- `FLUTTER_FILE_SELECTOR_VERSION` (`1.0.4`) and
  `flutter_pubspec_with_platform_effects` pin `file_selector` exactly in every
  generated pubspec. `flutter_pubspec_with_host_asset_dependencies` now leaves
  out a coordinate for a package the pubspec already declares (YAML refuses a
  duplicate key), so Engram's and photo-picker's `file_selector` range does
  not break `pub get`.
- `conformance/flutter-platform-effects/`: a Dart VM harness (fake host, fake
  dialogs, 277 checks) covering the Compose, SwiftUI and XAML cases plus a
  FIFO, a chosen link and a host disposed mid-dialog. It also covers the flag
  tables (checked against the running kernel), links and files planted at the
  temporary's name, the ownership rule, and a FIFO or device handed straight
  to the descriptor read;
  `tests/flutter_platform_effects.rs` runs it (with `dart analyze
  --fatal-infos`) wherever `dart` is installed. Unit tests pin the tables,
  ranges, messages, routing, imports and ordering to the Kotlin library.

