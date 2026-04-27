# Changelog

All notable changes to `coding-adventures-intel-4004-assembler` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-13

### Added

- Initial release: two-pass Intel 4004 assembler.

- **`lexer.py`** — `lex_line` / `lex_program` functions that tokenise assembly
  source lines into `ParsedLine` frozen dataclasses.  Handles labels, mnemonics,
  operands, inline comments, the `ORG` directive, and the `$` self-loop operand.

- **`encoder.py`** — `encode_instruction` encodes all 46 Intel 4004 instructions
  (plus the simulator-only `HLT` opcode and the `ADD_IMM` pseudo-instruction)
  into raw bytes.  `instruction_size` returns byte counts for Pass 1 use.
  `AssemblerError` is raised on unknown mnemonics, undefined labels, or
  out-of-range values.

- **`assembler.py`** — `Intel4004Assembler` class implements the classic two-pass
  algorithm: Pass 1 builds the symbol table; Pass 2 emits bytes.  The module-level
  `assemble(text) -> bytes` convenience function wraps the class.  `ORG` directives
  set the program counter and pad output with `0x00` bytes when advancing forward.

- **`__init__.py`** — exports `assemble`, `Intel4004Assembler`, `AssemblerError`.

- **Test suite** — 180 tests across three files:
  - `test_lexer.py`: blank lines, comments, label-only lines, instructions,
    inline comments, ORG directive, `$` operand, `lex_program` integration.
  - `test_encoder.py`: every instruction class and boundary values; unknown
    mnemonic errors; `instruction_size` for all groups; label/`$` resolution.
  - `test_assembler.py`: simple programs, label resolution, forward references,
    self-loops, full counter-loop program, subroutine call/return, error cases,
    ORG padding, reusable instance isolation.

- Coverage: 94.83% (exceeds 80% threshold).
- All code passes `ruff` linting with `E,W,F,I,UP,B,SIM,ANN` rules.
