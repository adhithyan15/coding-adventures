# Changelog

## [0.1.0] - 2026-03-18

### Added
- ARMDecoder: decodes MOV immediate, ADD register, SUB register, HLT
- ARMExecutor: executes decoded ARM instructions against CPU registers
- Assembler helpers: encode_mov_imm, encode_add, encode_sub, encode_hlt, assemble
- ARMSimulator: high-level wrapper combining CPU + ARM decoder/executor
- Knuth-style literate comments explaining ARM architecture and encoding
