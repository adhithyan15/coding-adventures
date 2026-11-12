# Changelog

## Unreleased

- Tests: the Pico UF2 mount tests now expect the platform's native separator
  before the volume label. The Rust discovery code returns `<root>\RPI-RP2` on
  Windows while Ruby's `File.join` builds `<root>/RPI-RP2`, so four assertions
  failed on Windows only (no production change; every assertion still checks
  the full mount path).

- Windows: the generated MSVC import library for the Ruby DLL now exports
  `rb_eval_string`. `ruby-bridge` obtains `nil`/`true` through it, so with the
  symbol missing (left unresolved by `/FORCE:UNRESOLVED`) the first native call
  that returned `nil` crashed Ruby with a segmentation fault in `detect_target`.

## 0.1.0

- Added the first Ruby Board VM DSL for Uno R4 WiFi sessions.
- Added `flash: true` support through the Rust SerialUSB artifact helper.
- Added `board.led.blink` and `board.eject.blink` helpers backed by the shared
  Rust CLI.
