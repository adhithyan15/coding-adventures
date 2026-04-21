# coding-adventures-intel-8008-ir-validator

Pre-flight hardware-constraint validation for Oct IR programs targeting the Intel 8008 CPU.

## What Is This?

This package is part of the **Intel 8008 backend** for the Oct compiler pipeline.
It runs immediately after `oct-ir-compiler` and before `ir-to-intel-8008-compiler`,
acting as a pre-flight check that confirms the IR program is physically feasible
on real Intel 8008 hardware.

```
Source text
    → oct-lexer
    → oct-parser
    → oct-type-checker
    → oct-ir-compiler          (typed AST → IrProgram)
    → intel-8008-ir-validator  ← this package
    → ir-to-intel-8008-compiler
    → intel-8008-assembler
    → intel-8008-packager
```

The Oct type checker enforces language-level rules (types, scopes, arities).
This validator enforces *hardware* rules that the type checker cannot know:

| What | Why the type checker can't catch it |
|------|-------------------------------------|
| No LOAD_WORD / STORE_WORD | Language has no 16-bit type |
| Static RAM ≤ 8 191 bytes | Hardware RAM size is invisible to the type system |
| Call depth ≤ 7 | Stack overflow is a runtime phenomenon |
| ≤ 6 virtual registers | Physical register count is a backend detail |
| Immediates 0–255 | Literal range isn't constrained by the language |
| Valid SYSCALL numbers | Port range is a hardware specification |

## Validation Rules

| Rule | Constraint |
|------|-----------|
| `no_word_ops` | No `LOAD_WORD` or `STORE_WORD` — 8008 is 8-bit only |
| `static_ram` | Total static data ≤ 8 191 bytes (RAM: 0x2000–0x3FFE) |
| `call_depth` | Static call graph depth ≤ 7 (8-level hardware stack; level 0 = current PC) |
| `register_count` | ≤ 6 distinct virtual registers (v0–v5: zero, scratch, 4 locals) |
| `imm_range` | `LOAD_IMM` and `ADD_IMM` immediates ∈ [0, 255] |
| `syscall_whitelist` | SYSCALL numbers ∈ {3,4} ∪ {11–16} ∪ {20–27} ∪ {40–63} |

All violations are accumulated in a single pass — you see every problem at once.

## SYSCALL Whitelist

The 8008 SYSCALL numbers and their hardware intrinsic mappings:

| SYSCALL | Oct intrinsic | 8008 instruction |
|---------|--------------|-----------------|
| 3 | `adc(a, b)` | `ADC r` |
| 4 | `sbb(a, b)` | `SBB r` |
| 11 | `rlc(a)` | `RLC` |
| 12 | `rrc(a)` | `RRC` |
| 13 | `ral(a)` | `RAL` |
| 14 | `rar(a)` | `RAR` |
| 15 | `carry()` | `MVI A,0; ACI 0` |
| 16 | `parity(a)` | `ORA A; JFP …` |
| 20–27 | `in(p)` for p ∈ 0–7 | `IN p` |
| 40–63 | `out(p, v)` for p ∈ 0–23 | `OUT p` |

## Usage

```python
from compiler_ir import IrProgram
from intel_8008_ir_validator import IrValidator, IrValidationError

# Obtain an IrProgram from oct-ir-compiler
prog: IrProgram = ...

validator = IrValidator()
errors = validator.validate(prog)

if errors:
    for e in errors:
        print(e)   # prints "[rule] message"
    raise RuntimeError("IR program is not feasible on Intel 8008 hardware")

# Safe to proceed to ir-to-intel-8008-compiler
```

`IrValidationError` also inherits from `Exception`, so the backend can raise it:

```python
errors = IrValidator().validate(prog)
if errors:
    raise errors[0]   # raise the first violation
```

## Intel 8008 Hardware Constraints

The Intel 8008 (1972) is an 8-bit microprocessor with:

- **Registers**: A (accumulator), B, C, D, E (user data), H, L (memory address)
- **Call stack**: 8-level hardware push-down; level 0 = current PC
- **RAM**: 8 KB at 0x2000–0x3FFF (8 192 bytes; 8 191 usable)
- **Input ports**: 8 (ports 0–7)
- **Output ports**: 24 (ports 0–23)
- **Immediate width**: 8 bits (0–255 for MVI, ADI, etc.)

## How It Fits in the Stack

```
Layer 7: compiler-ir              — IrProgram, IrInstruction, IrOp
Layer 8: oct-ir-compiler          — Oct typed AST → IrProgram
Layer 9: intel-8008-ir-validator  ← HERE — IrProgram feasibility check
Layer 10: ir-to-intel-8008-compiler (next) — IrProgram → 8008 assembly
```

## Building and Testing

```bash
./BUILD          # on Linux/macOS
BUILD_windows    # on Windows
```

Creates a `.venv`, installs `compiler-ir` and dev dependencies, and runs `pytest`
with coverage reporting.
