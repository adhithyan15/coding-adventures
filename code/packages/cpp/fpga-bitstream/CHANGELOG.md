# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `fpga-bitstream` crate, in
  namespace `ca::fpga`: an iCE40 IceStorm record-stream emitter.
- `part_specs`, `ClbConfig`, `FpgaConfig` (with a `std::map` of CLBs keyed by
  `(row, col)`), `BitstreamReport`, `emit_bitstream` (returns
  `std::pair<std::vector<uint8_t>, BitstreamReport>`), `cmd` (throws
  `std::length_error` on payload > 253), and `write_bin` (throws on file error).
- `std::map` iteration gives the `(row, col)` order for free, so the byte stream
  matches the Rust crate exactly and is deterministic.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert the exact bytes
  captured from the real Rust crate, plus determinism, overwrite semantics, and
  the cmd builder.
