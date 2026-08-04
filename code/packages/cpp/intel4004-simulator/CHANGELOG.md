# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `intel4004-simulator` crate in
  namespace `ca::intel4004_simulator`: a behavioral simulator for the Intel 4004
  (1971), the world's first commercial single-chip microprocessor.
- Models the 4-bit accumulator architecture: 16 registers (8 pairs), carry flag,
  byte-addressable ROM, data RAM (4 banks × 4 registers × 16 characters), RAM
  status nibbles, per-bank output ports, the ROM I/O port, and the 3-level
  wrapping hardware call stack. All 46 instructions with per-instruction `Trace`
  records (address, raw bytes, mnemonic, before/after accumulator + carry,
  `std::optional` second byte).
- `Simulator::run` returns a `std::vector<Trace>`; `step()` executes one
  instruction (throwing `std::runtime_error` where the Rust `step()` asserts
  `!halted`). State accessors, `reset`, `load_program`, and `encode_*` free
  functions (two-byte forms return a `std::pair`). Every ROM read is
  bounds-checked; verified clean under ASan + UBSan.
- 240 checks mirroring the crate's unit tests (every instruction, register
  pairs, RAM/status/port round-trips, subroutine nesting and stack wrap, ISZ
  loops, BCD arithmetic, exhaustive KBP decode, trace inspection, and worked
  programs) run under every ISO C++ compiler via the shared `iso-harness`.
