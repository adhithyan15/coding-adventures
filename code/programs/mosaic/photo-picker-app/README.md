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
This package was that first one. Every backend now answers `files.open`
from Mosaic's platform library instead (`UI87` §7), so the package carries no
host code of its own.

## Layout

```
src/PhotoPickerApp.mil          -- interface: status/picking slots, onPickPhoto emit
src/PhotoPickerApp.mll          -- layout: status text + "Pick a Photo" button
src/PhotoPickerApp.{light,dark}.msl -- styling (native controls pick up dark mode themselves)
mosaic-package.toml             -- exports; no [host_effects] or [host_assets]
```

## Every backend: Mosaic's platform library

This app carries no effect handler of its own. Every generated Compose,
SwiftUI, Qt, XAML and Flutter project gets Mosaic's platform library
(`MosaicPlatformEffects.kt` / `.swift` / `.{h,cpp}` / `.cs`,
`mosaic_platform_effects.dart`, UI87 §7), which answers `files.open` with the
same UI59 contract -- the native file dialog, the picked file's name, MIME
type and bytes, never its path -- so the generated entry point installs only
the library.

The handlers this app used to carry (`host/compose/PhotoPickerEffects.kt`,
`host/qt/PhotoPickerEffects.{h,cpp}`, `host/xaml/PhotoPickerEffects.cs`,
`host/flutter/PhotoPickerEffects.dart`) were retired for it (UI87 §7.4): the
app claims no effect kinds, so the router sends `files.open` to the library
and a package handler would never be reached. The Flutter handler's
`file_selector` `[host_assets]` coordinate went with it, because every
generated Flutter project pins `file_selector` for the library (UI87 §7.7).
Their designs are kept as a record in UI59 §4, §7, §10 and §13; UI87 §7.6 and
§7.7 describe what the XAML and Flutter libraries do instead.

## Testing

```
cargo test
```

`tests/package_compiles.rs` (mirrors `task-app`'s/`engram-app`'s own
harness), 3 tests:

1. `.mil`/`.mll`/both `.msl` themes compile, and the component's
   slots/emits match what's expected.
2. The manifest declares `PhotoPickerApp` as the sole export and no
   `[host_effects]` or `[host_assets]` entry on any backend (the platform
   library answers everywhere).
3. There is no handler on any backend: `host/compose/`, `host/qt/`,
   `host/xaml/`, `host/flutter/` and `host/` itself are gone.

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
- **Flutter**: no handler of its own any more; the platform library's
  `files.open` is exercised by its conformance harness in
  `mosaic-app-bindings` and the Flutter runtime CI lane (UI87 §7.7).
