# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `intel8008-simulator` crate in
  namespace `ca::intel8008_simulator`: a behavioral simulator for the Intel 8008
  (1972), the world's first 8-bit microprocessor.
- Models registers A/B/C/D/E/H/L, the M pseudo-register, four condition flags, a
  16 KiB address space, and the 8-level push-down call stack. Full instruction
  set with per-instruction `Trace` records (address, raw bytes, mnemonic,
  before/after A + flags, `std::optional` memory access).
- `Simulator::run` returns a `std::vector<Trace>`; `step()` executes one
  instruction (throwing `std::runtime_error` where the Rust `step()` returns
  `Result`). Register/flag accessors, I/O ports, `reset`, `load_program`.
  Verified clean under ASan + UBSan.
- 54 checks mirroring the crate's unit tests (arithmetic, logical, INR/DCR,
  rotates, call/return + nesting, RST, M-register memory, conditional jumps,
  parity, CMP, I/O, worked programs, and trace inspection) run under every ISO
  C++ compiler via the shared `iso-harness`.
