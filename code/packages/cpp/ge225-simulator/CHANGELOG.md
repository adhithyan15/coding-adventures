# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `ge225-simulator` crate in
  namespace `ca::ge225_simulator`: a fetch-decode-execute GE-225 CPU simulator.
- Free functions (`encode_instruction` / `decode_instruction` /
  `assemble_fixed` / `assemble_shift`, throwing `Error`; `pack_words` /
  `unpack_words`) and the `Simulator` class (`load_words`, `read_word` /
  `write_word`, `step` returning a `Trace` / `run`, typewriter + card reader,
  and accessors).
- The complete memory-reference / fixed / shift instruction set, including
  double-precision arithmetic and the elaborate shift family, ported with
  unsigned-based bit manipulation so it stays free of signed-overflow UB; clean
  under ASan + UBSan.
- 21 checks mirroring the crate's unit tests, run under every ISO C++ compiler
  via the shared `iso-harness`.
