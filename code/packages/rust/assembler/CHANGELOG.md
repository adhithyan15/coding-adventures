# Changelog

All notable changes to the `assembler` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `ArmOpcode` enum with standard ARM data processing opcodes: `And`, `Eor`, `Sub`, `Rsb`, `Add`, `Cmp`, `Orr`, `Mov`.
- `ArmInstruction` enum with variants:
  - `DataProcessing` for arithmetic/logic instructions with register or immediate operands.
  - `Load` and `Store` for memory access (`LDR`/`STR`).
  - `Nop` for no-operation.
  - `Label` for address markers.
- `Operand2` enum distinguishing register and immediate operands.
- `Assembler` struct with two-pass assembly:
  - `parse()`: First pass -- converts source text into structured `ArmInstruction`s, records labels.
  - `encode()`: Second pass -- converts instructions into 32-bit binary words.
- Register parsing supporting `R0`-`R15`, `SP`, `LR`, `PC` (case-insensitive).
- Immediate parsing supporting decimal (`#42`) and hexadecimal (`#0xFF`) formats.
- Comment stripping (`;` and `//` styles).
- `AssemblerError` enum with variants for unknown mnemonics, invalid registers, invalid immediates, and operand count mismatches.
