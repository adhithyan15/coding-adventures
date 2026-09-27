### Added — the SwiftUI platform library (UI87 §7)

`swift_platform_effects()` returns `MosaicPlatformEffects.swift`, which the
artifact builder writes into every SwiftUI project beside the runtime binding.
It answers `files.open` and `files.save` for every app with the Compose
library's contract, limits and MIME table (a Rust test pins the two templates
together):

- macOS: `NSOpenPanel` / `NSSavePanel`. Names, never paths; 50 MiB open,
  bounded while reading; 16 MiB save, checked on the encoded length first;
  the same plain-name rule (no separators, `:`, control or format characters,
  trailing dot or space); an extension of an accepted type when the app names
  one.
- The save is written to an owner-only temporary file created with `O_EXCL`
  beside the target, given the replaced file's rwx bits (never setuid, setgid
  or sticky; only from a regular file the person owns) with `fchmod` on the
  open descriptor, and `rename`d into place.
- `files.open` opens the chosen file once (`O_NONBLOCK | O_NOFOLLOW`, after
  resolving a symlink the person chose) and checks its type with `fstat` on
  that descriptor, so a FIFO swapped in cannot hang the main queue.
- iOS and iPadOS: no panels yet (UI89 step 6). A standard kind fails at once
  with "… is not available on this platform yet" rather than waiting. Every
  AppKit use is behind `#if os(macOS)`, which a Rust test checks.
- `MosaicPlatformRouter` routes by kind exactly as on Compose (claimed → app,
  standard → library, anything else → the app if it claimed nothing, else
  nobody); one file operation at a time; deferred, then answered on the main
  queue; the host is held weakly; installing twice on one host is a no-op.
- `conformance/swiftui` gains `PlatformEffectsChecks.swift`, run as
  `Conformance --platform-effects` with `-DMOSAIC_PLATFORM_EFFECTS` in the
  macOS lane. Without the flag the harness builds as before (the TaskApp
  release workflow uses it that way).

