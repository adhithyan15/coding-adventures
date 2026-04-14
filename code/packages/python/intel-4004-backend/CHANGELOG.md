# Changelog — coding-adventures-intel-4004-backend

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-12

### Added

**Phase 1 — IrValidator** (`validator.py`):
- `IrValidationError` frozen dataclass with `rule` and `message` fields
- `IrValidator.validate(program)` checks five hardware feasibility rules:
  - `no_word_ops`: rejects LOAD_WORD and STORE_WORD (no 16-bit bus on 4004)
  - `static_ram`: rejects programs with > 160 bytes of static data (4 × 4002 chips)
  - `call_depth`: rejects static call-graph depth > 2 (3-level hardware stack)
  - `register_count`: rejects programs using > 12 distinct virtual registers
  - `operand_range`: rejects LOAD_IMM values outside [0, 255]
- Accumulates all errors before returning (never stops at the first violation)
- DFS-based call-graph analysis with cycle detection for the call_depth rule

**Phase 2 — CodeGenerator** (`codegen.py`):
- `CodeGenerator.generate(program)` translates `IrProgram` → Intel 4004 assembly text
- Complete opcode coverage:
  - Constants: LOAD_IMM (LDM+XCH for k≤15, FIM for k≤255), LOAD_ADDR (FIM)
  - Memory: LOAD_BYTE (SRC+RDM+XCH), STORE_BYTE (LD+SRC+WRM)
  - Arithmetic: ADD, ADD_IMM, SUB, AND, AND_IMM
  - Comparison: CMP_LT (SUB+TCS), CMP_EQ (SUB+CMA+IAC), CMP_NE/CMP_GT (comment)
  - Control flow: JUMP (JUN), BRANCH_Z (LD+JCN 0x4), BRANCH_NZ (LD+JCN 0xC)
  - Subroutines: CALL (JMS), RET (BBL 0)
  - System: HALT (JUN $), SYSCALL (comment — not natively supported)
  - Meta: NOP, COMMENT
- Fixed virtual → physical register mapping (v0=R0, v1=R1, v2=R2, ..., v12=R12)
- Output format: `ORG 0x000` header, 4-space instruction indent, unindented labels

**Backend** (`backend.py`):
- `Intel4004Backend.compile(program)` — validates then generates in one call
- Raises `IrValidationError` with combined message if validation fails
- Module-level convenience functions: `validate()`, `generate_asm()`

**Tests** (`tests/`):
- `test_validator.py`: 30+ tests covering all 5 validation rules, accumulation, error type
- `test_codegen.py`: 40+ tests covering every opcode, indentation, register mapping
- `test_backend.py`: 20+ integration tests, exact snapshot assertions, error propagation
- Coverage > 95%
