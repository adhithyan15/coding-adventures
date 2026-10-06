# Flutter platform library conformance (UI87 §7.7)

This Dart console harness drives the Flutter platform library's core
(`mosaic_platform_effects_core.dart`: `files.open` / `files.save` and the
router that sends each effect to the app's handler or to the library by kind)
with a fake host and a fake dialog, so the real open/save logic, limits and
routing run on the plain Dart VM -- no display, no Flutter engine, no Rust
runtime. It checks the same cases as the Compose library's
`MosaicPlatformEffectsTest.kt`, the SwiftUI harness's
`PlatformEffectsChecks.swift` and the XAML harness:

- routing: a claimed kind to the app (even a standard one), an unclaimed
  standard kind to the library, a custom kind to the app when it claimed
  nothing and to nobody when it did (an empty `kinds` list claims nothing); a
  second install changes nothing;
- `files.save`: the bytes under the chosen name, the name (never a path)
  returned, replace-in-place with no temporary left behind, cancel, every
  refused name (paths, `:`, dot-files, RLO and other format characters,
  padding, trailing dot, lone surrogates, private use, invisible characters),
  a name that does not match the accepted type, executable extensions when no
  type is named, bad base64 (including the URL-safe alphabet and `%3D`, which
  Dart's own decoder would accept), oversized payloads, and -- on Unix -- that
  a replaced file keeps its permission bits (never setuid), a new one is 0600,
  and a chosen symlink is replaced without lending its target anything;
- `files.open`: name, MIME type and bytes, the accept list mapped to
  extensions in order, cancel, a directory, a missing file and a FIFO refused,
  a chosen symlink read and named as chosen, exactly 50 MiB accepted and one
  byte more refused;
- the libc calls behind both (POSIX): the `open(2)` flag table for each ABI
  pinned, and the running ABI's table probed with raw `open(2)` calls
  (`O_NOFOLLOW`, `O_CREAT | O_EXCL`, `O_NONBLOCK` on a FIFO, `O_CLOEXEC`); a
  link, a dangling link and a file planted at the save's temporary name each
  fail the save with nothing written, truncated or created through them; the
  mode rule (a file this user owns keeps its rwx bits, someone else's gives
  0600, an unknown owner loses group and other write); and a FIFO (under a
  deadline, so a regression fails instead of hanging), a device and a
  directory handed straight to the descriptor read are refused;
- the router's timing: deferred before any dialog, the dialog run from the
  scheduled work, one operation at a time, a request that outlives the
  payload it was copied from, a notify ignored, a refused deferral leaving it
  free, no dialogs on the platform (Android, iOS) answered with a clear
  failure, and every way the work can fail after deferral (a refusing
  scheduler, a throwing dialog, a disposed host) still ending in an answer
  or, for a disposed host, in nothing escaping;
- a host disposed while a dialog is open (the generated shell's retried
  start): the old router's late answer is dropped by the old host, never
  reaches the new one (which reuses the effect id), and does not hold the new
  router busy;
- the adapter over the generated `MosaicHost` with no runtime behind it.

The dialogs file (`mosaic_platform_effects.dart`, `package:file_selector`)
needs the Flutter engine and is not part of this harness; the Flutter CI lane
compiles it for real in every generated project it builds.

Run it against a generated project's files, which are not duplicated here:

```sh
mkdir -p lib
cp <generated>/flutter/lib/mosaic_host.dart \
   <generated>/flutter/lib/mosaic_platform_effects_core.dart lib/
dart pub get
dart run bin/conformance.dart
```

CI does this in the Flutter runtime lane against TaskApp's generated project,
and `tests/flutter_platform_effects.rs` does it wherever `dart` is installed,
against the files `mosaic-app-bindings` emits.
