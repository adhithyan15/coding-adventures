# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `ge225-simulator` crate: a fetch-decode-execute
  simulator for the GE-225 (1959), the mainframe Dartmouth BASIC was designed on.
- Instruction assembly (`ge225_encode_instruction`, `ge225_decode_instruction`,
  `ge225_assemble_fixed`, `ge225_assemble_shift`) and 3-byte word packing
  (`ge225_pack_words` / `ge225_unpack_words`).
- `Ge225Simulator` (opaque) with `ge225_new` / `ge225_free` / `ge225_reset`,
  `ge225_load_words`, `ge225_read_word` / `ge225_write_word`, `ge225_step` /
  `ge225_run`, the console typewriter and card reader, `ge225_disassemble_word`,
  and the full set of state accessors.
- The complete instruction set: memory-reference (LDA/ADD/SUB/STA, index and
  branch ops, double-precision DLD/DAD/DSU/DST/MPY/DVD, MOY/RCD/ORY/STO, …),
  fixed instructions, and the shift family (SRA/SLA/SCA/SAN/SNA/SRD/NAQ/SCD/
  ANQ/SLD/NOR/DNO).
- All left shifts and double-word bit-shuffling use unsigned arithmetic so the
  port is free of the signed-overflow UB a naive translation of Rust's wrapping
  shifts would introduce; clean under ASan + UBSan.
- 46 checks mirroring the crate's unit tests (encode/decode/pack round-trip,
  LDA/ADD/STA and SPB programs, odd-address double ops, MOY block moves, the
  console-typewriter path through SAN, card-reader RCD, disassembly, and a
  divide-by-zero error path), run under every ISO C compiler via the shared
  `iso-harness`.
