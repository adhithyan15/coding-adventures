# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `window-c`, a C ABI wrapper over the shared Rust windowing contract.
- C enums and POD structs mirroring surface preference, mount target, size, and
  AppKit render-target state.
- Opaque native window handles that currently expose the AppKit path on Apple
  platforms.
- Error propagation through `window_c_last_error_message()`.
- Unit tests covering attribute conversion and mount-target translation.
