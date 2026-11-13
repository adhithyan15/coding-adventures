# Changelog

## Unreleased

### Fixed

- `pico_uf2_mounts` (and so `pico_uf2_mount` and the flash command's `--mount`) hands mounts back in Ruby's path form: the Rust side joins with the host separator, so on Windows a mount came back as `D:/tmp/x\RPI-RP2` and four tests compared it against `File.join`-built paths with `/` (`test_pico_uf2_mounts_are_discovered_by_rust_language_core`, `..._selects_single_discovered_mount`, `..._reports_multiple_discovered_mounts`, `test_pico_flash_uses_auto_detected_bootsel_mount_by_default`). `File::ALT_SEPARATOR` is nil on Unix, where a backslash is an ordinary filename character, so Unix paths are untouched. This was the last Windows failure of this package after its native library started linking.

### Fixed

- Windows link: the `board_vm_native` `build.rs` import-library symbol list now includes `rb_eval_string` (used by `ruby-bridge`'s `nil_value`/`true_value`), which was the undefined symbol that failed the cdylib link on the Windows runner.

- `board_vm_native`'s `rb_i64` casts through `isize` for `ruby_bridge::rb_int2inum`, which now takes an `intptr_t` (the old `c_long` was 32 bits on 64-bit Windows).

## 0.1.0

- Added the first Ruby Board VM DSL for Uno R4 WiFi sessions.
- Added `flash: true` support through the Rust SerialUSB artifact helper.
- Added `board.led.blink` and `board.eject.blink` helpers backed by the shared
  Rust CLI.
