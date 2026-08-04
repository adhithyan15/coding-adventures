# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `intel-8008-assembler` crate in
  namespace `ca::intel8008_assembler`: a two-pass assembler that turns Intel
  8008 assembly text into raw machine-code bytes.
- Pass 1 builds the symbol table; Pass 2 encodes every instruction, resolving
  forward references, with `ORG` padding forward using `0xFF`. Full ISA subset
  (fixed 1-byte ops, `MOV`/`INR`/`DCR`/`IN`/`OUT`/`RST`, ALU-register and
  ALU-immediate, `MVI`, 3-byte jumps/calls); operands may be decimal/hex
  literals, `$`, labels, or `hi(sym)`/`lo(sym)`.
- `assemble(text)` plus the lower-level `instruction_size` and
  `encode_instruction`; `Symbols` is a `std::map<std::string, std::size_t>`.
  Exceptions (`AssemblerError : std::runtime_error`) replace the Rust `Result`.
  Verified clean under ASan + UBSan.
- 70 checks mirroring the crate's unit tests (sizes, per-instruction encoding,
  label/hi/lo resolution, full two-pass assembly, error paths) run under every
  ISO C++ compiler via the shared `iso-harness`.
