# Changelog

## Unreleased

### Fixed

- Windows link: the `board_vm_native` `build.rs` import-library symbol list now includes `rb_eval_string` (used by `ruby-bridge`'s `nil_value`/`true_value`), which was the undefined symbol that failed the cdylib link on the Windows runner.

- `board_vm_native`'s `rb_i64` casts through `isize` for `ruby_bridge::rb_int2inum`, which now takes an `intptr_t` (the old `c_long` was 32 bits on 64-bit Windows).

## 0.1.0

- Added the first Ruby Board VM DSL for Uno R4 WiFi sessions.
- Added `flash: true` support through the Rust SerialUSB artifact helper.
- Added `board.led.blink` and `board.eject.blink` helpers backed by the shared
  Rust CLI.
