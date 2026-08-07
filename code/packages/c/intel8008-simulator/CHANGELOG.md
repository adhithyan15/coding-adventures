# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `intel8008-simulator` crate: a behavioral
  simulator for the Intel 8008 (1972), the world's first 8-bit microprocessor.
- Models registers A/B/C/D/E/H/L, the M pseudo-register (memory at `[H:L]`),
  four condition flags (carry/zero/sign/parity), a 16 KiB address space, and the
  8-level push-down call stack. Full instruction set: MOV, MVI, INR/DCR,
  rotates, the eight ALU ops (register + immediate), conditional/unconditional
  jumps and calls, RST, RET, IN/OUT, HLT — with per-instruction `I8008Trace`
  records (address, raw bytes, disassembly, before/after A + flags, memory
  access).
- `i8008_new`/`_free`/`_reset`/`_load_program`, `i8008_run`/`i8008_step`,
  register/flag/state accessors, I/O ports, and `i8008_trace_count`/`i8008_trace`.
- The growable trace buffer guards `size_t` overflow. Verified clean under
  ASan + UBSan and the macOS `leaks` tool (0 leaks).
- 55 checks mirroring the crate's unit tests (arithmetic, logical, INR/DCR,
  rotates, call/return + nesting, RST, M-register memory, conditional jumps,
  parity, CMP, I/O, worked programs, and trace inspection) run under every ISO C
  compiler via the shared `iso-harness`.
