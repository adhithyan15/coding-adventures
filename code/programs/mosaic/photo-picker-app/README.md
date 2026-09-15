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
This package is that first one: XAML only in this slice (`UI59` §2) —
Qt, Compose, and Flutter follow as separate PRs, in the same order
`[host_effects]` itself landed in.

## Layout

```
src/PhotoPickerApp.mil          -- interface: status/picking slots, onPickPhoto emit
src/PhotoPickerApp.mll          -- layout: status text + "Pick a Photo" button
src/PhotoPickerApp.{light,dark}.msl -- styling (native controls pick up dark mode themselves)
host/xaml/PhotoPickerEffects.cs -- the XAML files.open handler ([host_effects])
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

## Testing

```
cargo test
```

`tests/package_compiles.rs` (mirrors `task-app`'s/`engram-app`'s own
harness):

1. `.mil`/`.mll`/both `.msl` themes compile, and the component's
   slots/emits match what's expected.
2. The manifest declares `PhotoPickerApp` as the sole export, the XAML
   `[host_effects]` file and handler exactly as documented (no
   `include` — the XAML emitter refuses one outright), and that no
   other backend (Qt/SwiftUI/Compose/Flutter) has a `[host_effects]`
   entry yet.
3. `PhotoPickerEffects.cs` exists and declares `Install()`,
   `MosaicRuntimeHost.EffectHandler`, and the `"files.open"` kind
   string.

### Real build (manual, not part of `cargo test`)

Verified against a real `dotnet build` of the emitted
`--profile native-complete` project — succeeds with 0 errors, 0
warnings. Interactively exercising the native `FileOpenPicker` dialog
itself isn't something this environment can automate; that gap is
stated explicitly rather than silently skipped (`UI59` §5).
