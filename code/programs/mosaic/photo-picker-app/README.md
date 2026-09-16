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
This package is that first one: XAML, Qt, and Compose so far (`UI59`
§2) — Flutter follows as the last separate PR, in the same order
`[host_effects]` itself landed in.

## Layout

```
src/PhotoPickerApp.mil          -- interface: status/picking slots, onPickPhoto emit
src/PhotoPickerApp.mll          -- layout: status text + "Pick a Photo" button
src/PhotoPickerApp.{light,dark}.msl -- styling (native controls pick up dark mode themselves)
host/xaml/PhotoPickerEffects.cs -- the XAML files.open handler ([host_effects])
host/qt/PhotoPickerEffects.{h,cpp} -- the Qt files.open handler ([host_effects])
host/compose/PhotoPickerEffects.kt -- the Compose files.open handler ([host_effects])
mosaic-package.toml             -- exports + [host_effects] wiring
```

## The XAML handler

`host/xaml/PhotoPickerEffects.cs`'s `PhotoPickerHost.PhotoPickerEffects
.Install()` sets `MosaicRuntimeHost.EffectHandler`, defers the effect
(`FileOpenPicker.PickSingleFileAsync()` is necessarily async; the
handler itself must return synchronously), and completes it as
`ok`/`cancelled`/`failed` once the user's picker interaction resolves.
Full design rationale — the namespace choice, the `GetForegroundWindow()`
owner-window approach, the MIME-type/extension mapping — is documented
inline in the file itself and in `UI59` §4.

Two non-obvious build errors this file's real `dotnet build` caught,
both now guarded by inline comments so a future edit doesn't
reintroduce them:

- **Namespace collision (CS0117).** The handler's C# namespace cannot
  be `PhotoPickerApp` — the generated component class is
  `Mosaic.Generated.PhotoPickerApp` (from this package's own
  `component PhotoPickerApp`), and the generated `Install();` call
  site lives inside `namespace Mosaic.Generated`, where an unqualified
  `PhotoPickerApp` resolves to that class first. The handler's
  namespace is `PhotoPickerHost` instead.
- **Lexical variable scoping (CS0136).** Two `var mimeType = ...`
  declarations in the same method — one building the request filter,
  one building the result's MIME type — collide even though their
  runtime lifetimes never overlap, because C# scoping is lexical
  across the whole method. The filter-building one is named
  `candidateMimeType`.

## The Qt handler

`host/qt/PhotoPickerEffects.cpp`'s `installPhotoPickerEffects(MosaicHost
&host)` connects to `MosaicHost::effectRequested` (`Qt::DirectConnection`
— a queued connection would let the host's sweep fail the effect as
unanswered before the dialog opened) and answers **inline** via
`QFileDialog::getOpenFileName`, which blocks synchronously — no
`deferEffect` needed, unlike XAML's necessarily-async picker. This
mirrors Engram's own Qt effect handler (`installEngramEffects`) in
structure, the one other real `[host_effects]` Qt handler in this repo.
Full design rationale is documented inline in the file itself and in
`UI59` §7.

Two things this handler deliberately does differently from Engram's Qt
precedent, both informed by `/security-review` findings against this
same effect kind's XAML implementation (PR #15218):

- **Bounded reads from the start.** Rather than checking
  `QFileInfo::size()` once before opening the file (which XAML's first
  cut did, and which `/security-review` found was TOCTOU — the check
  and the read are separate operations, so a file growing in between
  isn't actually bounded), the Qt handler reads in 64 KiB chunks and
  fails the moment the running total exceeds the 50 MiB cap. There was
  never a window where the check and the read could disagree.
- **Generic `failed.message`.** Never a raw `QFile::errorString()` or
  `std::exception::what()` — both can embed local filesystem paths,
  and `failed.message` is app-visible data. `installEngramEffects`
  does surface those directly for its own already-merged handler; this
  one doesn't, applying the same lesson XAML's security review taught.

## The Compose handler

`host/compose/PhotoPickerEffects.kt`'s `installPhotoPickerEffects(host:
MosaicRuntimeHost)` sets `host.effectHandler`, **defers** the effect
(`host.deferEffect(id)`), and runs the actual dialog + I/O work inside
`SwingUtilities.invokeLater { ... }` — deferred for two reasons
specific to this host (not because `JFileChooser` is async; it blocks
the same way Qt's `QFileDialog` does): the host's monitor is held
across the `effectHandler` call, so an inline modal dialog would hold
it for as long as it's open, and Compose state must be written from
the UI thread (EDT), which is where the generated app's props-changed
handler runs. This mirrors Engram's own Compose effect handler
(`installEngramEffects`) in structure, the one other real
`[host_effects]` Compose handler in this repo. Full design rationale
is documented inline in the file itself and in `UI59` §10.

Same two departures from Engram's precedent as the Qt handler, both
informed by `/security-review` findings against this effect kind's
earlier implementations:

- **Bounded reads from the start.** Rather than checking
  `File.length()` once before calling `File.readBytes()` (which
  Engram's own Compose handler does, and which XAML's first cut also
  did — found TOCTOU by `/security-review`), the Compose handler reads
  in 64 KiB chunks via `FileInputStream` and fails the moment the
  running total exceeds the 50 MiB cap.
- **Generic `failed.message`.** Never a raw `Exception.message` —
  `installEngramEffects` does surface `error.message ?: "..."` for its
  own already-merged handler; this one doesn't.

## Testing

```
cargo test
```

`tests/package_compiles.rs` (mirrors `task-app`'s/`engram-app`'s own
harness), 5 tests:

1. `.mil`/`.mll`/both `.msl` themes compile, and the component's
   slots/emits match what's expected.
2. The manifest declares `PhotoPickerApp` as the sole export, the XAML
   `[host_effects]` file and handler exactly as documented (no
   `include` — the XAML emitter refuses one outright), the Qt
   `[host_effects]` files (header + source) and handler (with
   `include`), the Compose `[host_effects]` file and handler (no
   `include` — the Compose emitter refuses one outright too, for a
   different reason: Kotlin has no include directive), and that no
   other backend (SwiftUI/Flutter) has a `[host_effects]` entry yet.
3. `PhotoPickerEffects.cs` exists and declares `Install()`,
   `MosaicRuntimeHost.EffectHandler`, and the `"files.open"` kind
   string.
4. `PhotoPickerEffects.h`/`.cpp` exist and declare
   `installPhotoPickerEffects(MosaicHost &host)`, `effectRequested`,
   and the `"files.open"` kind string.
5. `PhotoPickerEffects.kt` exists and declares
   `installPhotoPickerEffects(host: MosaicRuntimeHost)`,
   `effectHandler`, and the `"files.open"` kind string.

### Real builds (manual, not part of `cargo test`)

- **XAML**: a real `dotnet build` of the emitted `--profile
  native-complete` project — succeeds with 0 errors, 0 warnings.
  Interactively exercising the native `FileOpenPicker` dialog itself
  isn't something this environment can automate; that gap is stated
  explicitly rather than silently skipped (`UI59` §5).
- **Qt**: a real `cmake --build` (Ninja generator, MSVC 19.44 via
  `vcvars64.bat`, Qt 6.8.1) of the emitted `--profile native-complete`
  project — succeeds with 0 errors (the only warning is in the
  generated `MosaicHost.cpp` template, not this package's own code).
  Same limitation on interactively exercising `QFileDialog` (`UI59`
  §8).
- **Compose**: a real `gradle build` (Gradle 8.10.2, JDK 21) of the
  emitted `--profile native-complete` project — succeeds with 0 errors
  (the one warning, "No cast needed" on the generated `Main.kt`'s
  splice line, is in generated scaffolding, not this package's own
  code). Same limitation on interactively exercising `JFileChooser`
  (`UI59` §11).
