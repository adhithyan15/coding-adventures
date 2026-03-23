# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Two-pass assembler (`Assembler` class and `assemble()` convenience function)
- Parser that handles labels, instructions, directives, and comments
- Instruction encoder for ARM data processing, branch, and memory formats
- Individual encoding functions: `encodeMovImm`, `encodeAdd`, `encodeSub`, `encodeHlt`
- Generic encoding functions: `encodeDataProcessing`, `encodeBranch`, `encodeMemory`
- ARM immediate encoding with rotation support (`encodeImmediate`)
- Full condition code support (EQ, NE, GT, LT, GE, LE, AL, etc.)
- Register aliases (SP=R13, LR=R14, PC=R15)
- Label resolution with forward and backward references
- Symbol table generation (label -> address mapping)
- Source map generation (address -> source line number mapping)
- Error collection (reports all errors, doesn't stop at first)
- Support for data processing instructions: MOV, MVN, ADD, SUB, AND, ORR, EOR, CMP, CMN, TST, TEQ
- Support for branch instructions: B, BL, with all condition suffixes
- Support for memory instructions: LDR, STR with register base and immediate offset
- Support for special instructions: HLT, NOP
- `instructionsToBytes()` helper for manual instruction encoding
- Comprehensive test suite with 95%+ coverage
- Literate programming style with extensive inline documentation
