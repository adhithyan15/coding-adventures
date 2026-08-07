# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `intel-4004-assembler` crate: a two-pass
  assembler for the Intel 4004.
- `i4004_assemble(text, &out, &out_len, err, err_len)` producing malloc'd machine
  code with an `I4004Status` result and a caller-supplied error message buffer.
- Lexer (labels / mnemonics / comma operands / `;` comments), pass-1 symbol
  table with `ORG`, pass-2 encoding with forward-`ORG` zero padding, and the full
  instruction table (NOP/HLT/WRM/LDM/BBL/INC/ADD/SUB/LD/XCH/SRC/FIN/JIN one-byte;
  JCN/FIM/JUN/JMS/ISZ/ADD_IMM two-byte).
- Overflow-guarded growable byte buffer and dynamic line/symbol tables.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the Rust reference
  vector plus hand-computed encodings for every instruction, labels, ORG
  padding, comments, and error cases.
