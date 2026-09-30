### Added — the XAML platform library (UI87 §7.6)

- `xaml_platform_effects(namespace)` returns `MosaicPlatformEffects.cs`:
  `files.open` and `files.save` through WinUI 3's `FileOpenPicker` /
  `FileSavePicker`, initialised with the owning window's handle
  (`InitializeWithWindow`, which an unpackaged WinUI 3 app needs), with the
  Compose library's contract -- limits (50 MiB open, bounded while reading;
  16 MiB save, checked on the encoded length first), the MIME table in the
  same order, the plain-name rule (by code point: separators, `:`,
  dot-files, control/format/separator characters, lone surrogates,
  unassigned and private-use code points, padding and trailing dots with
  blanks as whitespace and invisible characters dropped first), the
  executable list and every failure message. A name is returned, never a
  path.
- Saves go to an exclusive (`CreateNew`) temporary beside the target,
  owner-only on Unix and given the replaced file's rwx bits through the open
  handle, flushed to disk, then `File.Move(overwrite: true)` into place. The
  base64 alphabet is checked before decoding, since .NET's decoder skips the
  whitespace the Compose and SwiftUI decoders refuse.
- `MosaicPlatformEffects.Install(host, appKinds, dialogs, runOnUi)` wraps the
  app's `EffectHandler` in a `MosaicPlatformRouter`: claimed kinds to the
  app, unclaimed standard kinds here, anything else to the app when it
  claimed nothing and to nobody otherwise; idempotent. Each standard effect
  is deferred, its payload cloned, and the picker started from the UI queue
  after the settle; a refused or throwing queue, a throwing picker and a
  closed host all end without leaving the effect pending. One file operation
  at a time. `Install(window, appKinds)` is the WinUI entry point the
  generated window calls.
- The static host is reached through `IMosaicPlatformEffectHost`
  (`MosaicRuntimeHostEffects` forwards to `MosaicRuntimeHost`); the pickers
  through `IMosaicFileDialogs`. Everything WinUI sits inside
  `#if !MOSAIC_HEADLESS_TEST`, which no generated project defines.
- `conformance/xaml-platform-effects/`: a headless .NET harness (fake host,
  fake picker) with the Compose test's and the SwiftUI harness's cases --
  routing, save/replace/permissions, every refused name, type mismatch,
  executables, base64 and size limits, open/cancel/non-file/oversize,
  symlinks, busy, notify, refused deferral, a payload outliving its document,
  and each failure after deferral. `tests/xaml_platform_effects.rs` builds
  and runs it against the emitted files wherever `dotnet` is installed.
- Rust tests pin the kinds, limits, MIME rows, executable list, messages and
  routing to the Kotlin library, and every WinUI use to the fence.

