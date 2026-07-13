# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `zwave-core` crate in namespace
  `ca::zwave_core`: Z-Wave identifier, region, and Serial API frame primitives.
- `NodeId`, `home_id_to_be_bytes`, `RegionProfile` helpers, and `CommandClassId`
  (encoding + classification, with named constants).
- `CommandClassFrame` / `SerialFrame` with `parse` (throws `Error` on bad input)
  and `encode`; the serial framing validates SOF, length, type, and the XOR
  checksum. `Error` carries an `ErrorKind` plus `a()`/`b()` parametric detail.
- Network, command-class-frame, serial-frame-batch, and controller-readiness
  summaries with value equality and boolean helpers.
- 45 checks mirroring the crate's unit tests, run under every ISO C++ compiler
  via the shared `iso-harness`; also clean under ASan + UBSan.
