# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-15

### Added

- Initial Mosaic package: `PhotoPickerApp` (`.mil`/`.mll`/`.msl`) — a
  status line and a "Pick a Photo" button, exercising `UI59`'s
  `files.open` effect end to end.
- `host/xaml/PhotoPickerEffects.cs` — the first real, non-Engram-
  specific `[host_effects]` handler on any backend, implementing
  `files.open` via WinUI 3's `FileOpenPicker`.
- `[host_effects]` manifest wiring for XAML only, per `UI59` §2; Qt,
  Compose, and Flutter are explicit follow-up PRs.
- `tests/package_compiles.rs` — compile-check + manifest-shape
  coverage, mirroring `task-app`/`engram-app`'s own harness.
- Verified with a real `dotnet build` of the `--profile
  native-complete` emitted project (0 errors, 0 warnings), not only
  generated-text assertions.

### Fixed (caught by the real `dotnet build`, before this ever shipped)

- CS0117: the handler's namespace collided with the generated
  `Mosaic.Generated.PhotoPickerApp` component class — renamed to
  `PhotoPickerHost`.
- CS0103: `MosaicRuntimeHost` wasn't visible from the renamed
  namespace — added the missing `using Mosaic.Generated;`.
- CS0136: two `var mimeType` locals in the same method collided under
  C#'s lexical (not lifetime-based) scoping — the request-filter one
  renamed to `candidateMimeType`.
