# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `intel4004-simulator` crate: a behavioral
  simulator for the Intel 4004 (1971), the world's first commercial single-chip
  microprocessor.
- Models the 4-bit accumulator architecture faithfully: 16 registers (8 pairs),
  carry flag, byte-addressable ROM, data RAM (4 banks × 4 registers × 16
  characters), RAM status nibbles, per-bank output ports, the ROM I/O port, and
  the 3-level wrapping hardware call stack. All 46 instructions — JCN/JUN/JMS/
  BBL/ISZ control flow, FIM/FIN/SRC/JIN register-pair and RAM addressing, the
  accumulator ALU ops (with the inverted-carry subtraction convention), rotates,
  BCD `DAA`, and one-hot `KBP` — with per-instruction `I4004Trace` records
  (address, raw bytes, disassembly, before/after accumulator + carry).
- `i4004_new`/`_free`/`_reset`/`_load_program`, `i4004_run`/`i4004_step`, the
  full set of state accessors, `i4004_encode_*` machine-code helpers, and
  `i4004_trace_count`/`i4004_trace`.
- The growable trace buffer guards `size_t` overflow, and every ROM read is
  bounds-checked (a runaway program counter reads NOP rather than out of
  bounds). Verified clean under ASan + UBSan, the macOS `leaks` tool (0 leaks),
  and an all-opcodes fuzz sweep across ROM sizes.
- 243 checks mirroring the crate's unit tests (every instruction, register
  pairs, RAM/status/port round-trips, subroutine nesting and stack wrap, ISZ
  loops, BCD arithmetic, exhaustive KBP decode, trace inspection, and worked
  programs) run under every ISO C compiler via the shared `iso-harness`.
