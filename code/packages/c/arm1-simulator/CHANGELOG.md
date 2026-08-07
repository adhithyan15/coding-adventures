# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `arm1-simulator` crate: a complete behavioral
  simulator for the ARM1 (1985), the first ARM chip.
- The full ARMv1 instruction set — 16 data-processing ops through the inline
  barrel shifter (LSL/LSR/ASR/ROR/RRX), load/store (pre/post-indexed), block
  transfer (LDM/STM, four stacking modes), branch (B/BL), SWI, conditional
  execution (16 codes), and 4 processor modes with banked registers and the
  shared PC+status R15.
- `arm1_new`/`_free`/`_reset`, register/flag/mode/memory accessors,
  `arm1_step`/`_run` with per-instruction `Arm1Trace` records, and pure
  functions `arm1_evaluate_condition`/`_barrel_shift`/`_decode_immediate`/
  `_alu_execute`/`_decode`/`_disassemble` plus the `arm1_encode_*` helpers.
- `Arm1Trace` is a plain value type (fixed memory-access arrays — a block
  transfer touches ≤ 16 registers — and a bounded mnemonic buffer). Verified
  clean under ASan + UBSan, the macOS `leaks` tool (0 leaks), and a
  50k-iteration random-program / random-instruction fuzz.
- 107 checks mirroring the crate's unit tests (condition evaluation, barrel
  shifter, ALU, decode/disassemble, and worked programs: loops, LDR/STR,
  STM/LDM, branch-with-link, CMP + conditional branch, SWI) run under every ISO
  C compiler via the shared `iso-harness`.
