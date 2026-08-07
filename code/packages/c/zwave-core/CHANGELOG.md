# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `zwave-core` crate: Z-Wave identifier, region,
  and Serial API frame primitives (not a controller).
- `ZWaveNodeId` (classic / long-range, range-validated), `zw_home_id_to_be_bytes`,
  `ZWaveRegionProfile` (band descriptions + long-range support), and
  command-class id encoding (1/2-byte) + actuator/sensor/security classification.
- `ZWaveCommandClassFrame` and `ZWaveSerialFrame` with `_init` / `_parse` /
  `_encode` / `_free` — both parsers bounds-check untrusted bytes and report a
  structured `ZWaveError`; the serial framing validates SOF, length, type, and
  the XOR checksum (`zw_serial_checksum`).
- Network, command-class-frame, serial-frame-batch, and controller-readiness
  summaries with their boolean helpers.
- 57 checks mirroring the crate's unit tests (node-id ranges, frame round-trips
  for short/extended command classes, serial round-trip + checksum mismatch,
  truncation, and the readiness roll-up), run under every ISO C compiler via the
  shared `iso-harness`; also clean under ASan + UBSan.
