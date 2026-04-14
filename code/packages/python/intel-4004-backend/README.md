# coding-adventures-intel-4004-backend

A two-phase backend that takes a generic `IrProgram` (from `compiler-ir`) and
produces Intel 4004 assembly text.

This is PR 9 in the Nib language → Intel 4004 compiler pipeline.

## What is the Intel 4004?

The Intel 4004 (1971) is the world's first commercially available single-chip
CPU.  It is a 4-bit microprocessor with:

- 16 × 4-bit registers (R0–R15), grouped into 8 pairs for 8-bit operations
- 3-level hardware call stack
- 4 × Intel 4002 RAM chips = 160 bytes addressable RAM
- ~45 instructions (LDM, FIM, ADD, SUB, JCN, JUN, JMS, BBL, etc.)

## Pipeline position

```
Nib source
    ↓ (lexer + parser)
AST
    ↓ (type checker)
IrProgram       ← compiler-ir package
    ↓ (Phase 1: IrValidator)
Validated IrProgram
    ↓ (Phase 2: CodeGenerator)
Intel 4004 assembly text    ← this package
    ↓ (assembler)
Object code / ROM image
```

## Installation

```bash
pip install coding-adventures-intel-4004-backend
```

## Usage

### Quick start

```python
from compiler_ir import IrProgram, IrInstruction, IrOp, IrImmediate
from compiler_ir import IrRegister, IrLabel, IDGenerator
from intel_4004_backend import Intel4004Backend, IrValidationError

gen = IDGenerator()
prog = IrProgram(entry_label="_start")
prog.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")],         id=-1))
prog.add_instruction(IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=gen.next()))
prog.add_instruction(IrInstruction(IrOp.HALT,     [],                          id=gen.next()))

backend = Intel4004Backend()
try:
    asm = backend.compile(prog)
    print(asm)
except IrValidationError as e:
    print(f"Hardware constraint violated: {e}")
```

Output:

```asm
    ORG 0x000
_start:
    LDM 5
    XCH R2
    JUN $
```

### Standalone validation

```python
from intel_4004_backend import validate, IrValidationError

errors = validate(prog)
for e in errors:
    print(e)   # [rule_name] description of violation
```

### Standalone assembly generation

```python
from intel_4004_backend import generate_asm

asm = generate_asm(prog)   # skips validation — call validate() first
```

## Phase 1: IrValidator

The validator checks five hardware feasibility rules:

| Rule | Constraint |
|------|-----------|
| `no_word_ops` | No `LOAD_WORD` or `STORE_WORD` — 4004 has no 16-bit bus |
| `static_ram` | Total `IrDataDecl` sizes ≤ 160 bytes (4 × 4002 chips) |
| `call_depth` | Static call-graph DFS depth ≤ 2 (3-level hardware stack) |
| `register_count` | Distinct virtual register indices ≤ 12 |
| `operand_range` | Every `LOAD_IMM` immediate fits in u8 (0–255) |

All violations are collected before returning so you see every problem at once.

## Phase 2: CodeGenerator

Translates each IR opcode to Intel 4004 instructions:

| IR opcode | 4004 assembly |
|-----------|--------------|
| `LOAD_IMM vN, k` (k ≤ 15) | `LDM k` + `XCH Rn` |
| `LOAD_IMM vN, k` (k ≤ 255) | `FIM Pn, k` |
| `LOAD_ADDR vN, lbl` | `FIM Pn, lbl` |
| `LOAD_BYTE dst, base, off` | `SRC Pbase` + `RDM` + `XCH Rdst` |
| `STORE_BYTE src, base, off` | `LD Rsrc` + `SRC Pbase` + `WRM` |
| `ADD vR, vA, vB` | `LD Ra` + `ADD Rb` + `XCH Rr` |
| `SUB vR, vA, vB` | `LD Ra` + `SUB Rb` + `XCH Rr` |
| `CMP_LT vR, vA, vB` | `LD Ra` + `SUB Rb` + `TCS` + `XCH Rr` |
| `CMP_EQ vR, vA, vB` | `LD Ra` + `SUB Rb` + `CMA` + `IAC` + `XCH Rr` |
| `BRANCH_Z vN, lbl` | `LD Rn` + `JCN 0x4, lbl` |
| `BRANCH_NZ vN, lbl` | `LD Rn` + `JCN 0xC, lbl` |
| `JUMP lbl` | `JUN lbl` |
| `CALL lbl` | `JMS lbl` |
| `RET` | `BBL 0` |
| `HALT` | `JUN $` |
| `NOP` | `NOP` |

## Physical register mapping

| Virtual | Physical | Notes |
|---------|----------|-------|
| v0 | R0 | Zero constant (kept 0) |
| v1 | R1 | Scratch (internal use) |
| v2 | R2 | u4 scalar |
| v3 | R3 | u4 scalar |
| v4, v5 | R4:R5 (P2) | u8 variable |
| v6, v7 | R6:R7 (P3) | u8 variable |
| v8–v11 | R8–R11 | General purpose |
| v12 | R12 | RAM address (SRC) |

## Running tests

```bash
cd code/packages/python/intel-4004-backend
mise exec -- uv run pytest
```

## Package structure

```
src/intel_4004_backend/
    __init__.py    ← exports: validate, generate_asm, Intel4004Backend, IrValidationError
    validator.py   ← IrValidator: validate(program) → list[IrValidationError]
    codegen.py     ← CodeGenerator: generate(program) → str
    backend.py     ← Intel4004Backend: compile(program) → str (raises on validation failure)
tests/
    test_validator.py   ← one test class per validation rule
    test_codegen.py     ← one test class per IR opcode
    test_backend.py     ← integration tests
```
