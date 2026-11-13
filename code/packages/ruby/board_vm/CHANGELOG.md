# Changelog

## Unreleased

### Fixed

- `board_vm_native`'s `rb_i64` casts through `isize` for `ruby_bridge::rb_int2inum`, which now takes an `intptr_t` (the old `c_long` was 32 bits on 64-bit Windows).

## 0.1.0

- Added the first Ruby Board VM DSL for Uno R4 WiFi sessions.
- Added `flash: true` support through the Rust SerialUSB artifact helper.
- Added `board.led.blink` and `board.eject.blink` helpers backed by the shared
  Rust CLI.
