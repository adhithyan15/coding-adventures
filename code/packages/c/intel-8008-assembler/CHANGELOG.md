# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `intel-8008-assembler` crate: a two-pass
  assembler that turns Intel 8008 assembly text into raw machine-code bytes.
- Pass 1 builds the symbol table (label → address); Pass 2 encodes every
  instruction, resolving forward references, with `ORG` padding forward using
  `0xFF`. Covers fixed 1-byte ops, `MOV`/`INR`/`DCR`/`IN`/`OUT`/`RST`,
  ALU-register and ALU-immediate, `MVI`, and 3-byte jumps/calls; operands may be
  decimal/hex literals, `$`, labels, or `hi(sym)`/`lo(sym)`.
- `intel8008_assemble` plus the lower-level `intel8008_instruction_size` /
  `intel8008_encode_instruction` and an opaque `Intel8008Symbols` table. An
  `Intel8008Status` + caller `errbuf` replaces the Rust `Result`.
- All growable buffers guard `size_t` overflow; numeric parsing guards
  overflow. Verified clean under ASan + UBSan and the macOS `leaks` tool
  (0 leaks).
- 108 checks mirroring the crate's unit tests (lexer, sizes, per-instruction
  encoding, label/hi/lo resolution, full two-pass assembly, error paths) run
  under every ISO C compiler via the shared `iso-harness`.
