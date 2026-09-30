# `photo-picker-app` — reference Mosaic app for `files.open` (UI59)

A single-screen Mosaic app: a status line and one button, "Pick a
Photo." Clicking it asks the host to open its native file/gallery
picker via the `files.open` effect (`code/specs/UI59-files-open-
effect.md`) and renders back the picked file's name, MIME type, and
exact byte count. The app-logic side (what happens on `pickPhoto`,
how the effect result is rendered) lives in the separate
`photo-picker-mosaic-app` Rust crate — see
`code/packages/rust/photo-picker-mosaic-app/README.md`.

## Why this package exists

`UI47`'s `[host_effects]` mechanism wires a package-declared handler
into each backend's generated entry point, but until now no *generic*
(non-Engram-specific) effect kind had a real handler on any backend.
This package is that first one. Flutter answers `files.open` with this
package's own handler (`UI59` §2); Compose, SwiftUI, Qt and XAML answer it
from Mosaic's platform library (`UI87` §7).

## Layout

```
src/PhotoPickerApp.mil          -- interface: status/picking slots, onPickPhoto emit
src/PhotoPickerApp.mll          -- layout: status text + "Pick a Photo" button
src/PhotoPickerApp.{light,dark}.msl -- styling (native controls pick up dark mode themselves)
host/flutter/PhotoPickerEffects.dart -- the Flutter files.open handler ([host_effects])
mosaic-package.toml             -- exports + [host_effects]/[host_assets] wiring
```

## Compose, SwiftUI, Qt and XAML: Mosaic's platform library

This app carries no Compose, SwiftUI, Qt or XAML handler. Every generated
Compose, SwiftUI, Qt and XAML project gets Mosaic's platform library
(`MosaicPlatformEffects.kt` / `.swift` / `.{h,cpp}` / `.cs`, UI87 §7), which
answers `files.open` with the same UI59 contract — the native file dialog, the
picked file's name, MIME type and bytes, never its path — so the generated
entry point installs only the library. The Compose, Qt and XAML handlers this
app used to carry (`host/compose/PhotoPickerEffects.kt`,
`host/qt/PhotoPickerEffects.{h,cpp}`, `host/xaml/PhotoPickerEffects.cs`) were
retired for it (UI87 §7.4): the app claims no effect kinds, so the router
sends `files.open` to the library and a package handler would never be
reached. The XAML handler's design notes (the namespace choice, the owner
window, the MIME mapping) live on in UI59 §4, which the library follows. The
Flutter handler follows when that backend's library lands.

## The Flutter handler

`host/flutter/PhotoPickerEffects.dart`'s
`installPhotoPickerEffects(MosaicHost host)` sets `host.effectHandler`,
**defers** the effect (`host.deferEffect(id)`), and runs the dialog +
I/O inside an unawaited async closure — deferred for the simplest and
most inescapable reason of the backends it had: `openFile` returns a
`Future`, and `effectHandler` is a synchronous callback, so there is
no answer to give inline at all. Unlike the retired Qt and Compose handlers
there's nothing to marshal either: a Dart isolate is single-threaded, so the eventual
`completeEffect` call never races anything — it just arrives on a
later turn of the event loop. This mirrors Engram's own Flutter effect
handler (`installEngramEffects`) in structure, the one other real
`[host_effects]` Flutter handler in this repo. Full design rationale
is documented inline in the file itself and in `UI59` §13.

This is the only backend needing a new manifest table:
`package:file_selector` (the pub package that opens the platform file
dialog) isn't a default Flutter project dependency, so
`mosaic-package.toml` gains a `[host_assets].dependencies` entry —
this package's only `[host_assets]` content (no `.files` entries,
unlike Engram, which has no legacy host file to retire here).

Same two departures from Engram's precedent as the retired Qt and Compose
handlers made:

- **Bounded reads from the start.** Rather than checking
  `FileStat.size` once via `File.stat()` before calling
  `readAsBytes()` (which Engram's own Flutter handler does, the same
  TOCTOU-vulnerable shape XAML's first cut used), the Flutter handler
  opens a `RandomAccessFile` and reads in 64 KiB chunks, failing once
  the running total exceeds the 50 MiB cap.
- **Generic `failed.message`.** Never Engram's own `_reason(error,
  fallback)` helper's `error.toString()` output — for a
  `FileSystemException` that routinely embeds the full local
  filesystem path.

One thing this handler *doesn't* need to fix, unlike Compose: Engram's
own Flutter handler already catches `on Object` (not a narrower
`Exception`-only type) at every completion point — Dart's `catch` has
no Exception-vs-Error split the way Kotlin's does, so there was never
a narrower clause here to widen.

## Testing

```
cargo test
```

`tests/package_compiles.rs` (mirrors `task-app`'s/`engram-app`'s own
harness), 4 tests:

1. `.mil`/`.mll`/both `.msl` themes compile, and the component's
   slots/emits match what's expected.
2. The manifest declares `PhotoPickerApp` as the sole export, the Flutter
   `[host_effects]` file and handler (with `include`, since Dart resolves
   nothing across files without one) plus the `[host_assets].dependencies`
   entry for `file_selector`, and that none of Compose, SwiftUI, Qt or XAML
   has a `[host_effects]` entry (the platform library answers there).
3. There is no Compose, Qt or XAML handler: `host/compose/`, `host/qt/` and
   `host/xaml/` are gone.
4. `PhotoPickerEffects.dart` exists and declares
   `installPhotoPickerEffects(MosaicHost host)`, `effectHandler`, and
   the `'files.open'` kind string.

### Real builds (manual, not part of `cargo test`)

- **XAML**: no handler of its own any more; the platform library's
  `files.open` is exercised by its headless conformance harness in
  `mosaic-app-bindings` (UI87 §7.6).
- **Qt**: no handler of its own any more; the platform library's
  `files.open` is exercised by the Qt effect driver in
  `mosaic-app-bindings` (UI87 §7.4a).
- **Compose**: a real `gradle build` (Gradle 8.10.2, JDK 21) of the
  emitted `--profile native-complete` project — succeeds with 0 errors
  (the one warning, "No cast needed" on the generated `Main.kt`'s
  splice line, is in generated scaffolding, not this package's own
  code). Same limitation on interactively exercising `JFileChooser`
  (`UI59` §11).
- **Flutter**: `flutter pub get` (resolves `file_selector` and its
  platform plugins cleanly), `flutter analyze` (0 issues), and
  `flutter test` (the generated smoke test passes) on the emitted
  `--profile native-complete` project — all real, unqualified
  successes on the actual Flutter 3.47.0 toolchain installed here.
  `flutter build windows` itself wasn't exercised: Flutter's Windows
  plugin build needs OS-level symlink support gated behind Windows
  Developer Mode, which isn't enabled in this environment — a
  system-settings change out of scope for what this session enables
  on its own, stated explicitly rather than silently skipped (`UI59`
  §14). Same limitation on interactively exercising the platform file
  dialog as the other three backends.
