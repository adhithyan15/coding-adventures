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

### Fixed (caught by `/security-review`, before this ever shipped)

- **Unbounded file read (resource exhaustion).** The handler read a
  picked file fully into memory and base64-encoded it with no size
  cap — a single pick of an arbitrarily large file cost an
  arbitrarily large amount of host memory. Now checks
  `StorageFile.GetBasicPropertiesAsync().Size` against a 50 MiB cap
  and completes the effect as `failed` before reading, rather than
  after. This is exactly the concern `UI59` §6 acceptance gate 4
  flagged as worth a specific look. A round-2 review pass found the
  pre-check alone was TOCTOU (the file can grow between the check and
  the read) — `ReadAllBytesAsync` now enforces the same cap while
  actually copying, not only beforehand.
- **Raw exception messages in `failed.message`.** Any exception
  (including `UnauthorizedAccessException`, whose `.Message`
  routinely embeds the full local filesystem path) was surfaced
  verbatim through the effect's app-visible `failed.message` field.
  Now mapped to short, generic messages instead.
