# ir-to-intel-8008-compiler

Translates a validated `IrProgram` (from `compiler-ir`) into Intel 8008 assembly text.

## Pipeline Position

```
oct-lexer → oct-parser → oct-type-checker → oct-ir-compiler
  → intel-8008-ir-validator → ir-to-intel-8008-compiler  ← YOU ARE HERE
  → intel-8008-assembler → intel-8008-packager
```

## What it does

The package lowers every IR opcode to the appropriate Intel 8008 instruction
sequence.  The Intel 8008 (1972) is an 8-bit CPU with six 8-bit registers
(A, B, C, D, E, H, L) and a 14-bit address space.

### Virtual → Physical Register Mapping

| Virtual | Physical | Role |
|---------|----------|------|
| v0 | B | zero constant / scratch |
| v1 | C | scratch / return value |
| v2 | D | 1st local / 1st syscall arg |
| v3 | E | 2nd local / 2nd syscall arg |
| v4 | H | 3rd local (also memory high byte) |
| v5 | L | 4th local (also memory low byte) |
| A | (implicit) | accumulator — never a virtual register |

### Key Code-Generation Decisions

- **Memory access** — `LOAD_ADDR` sets H:L; `LOAD_BYTE`/`STORE_BYTE` use M
  (the pseudo-register for the byte at H:L).
- **NOT** — implemented as `XRI 0xFF` (no dedicated NOT on the 8008).
- **Comparisons** — materialise a boolean into a register via a 6-instruction
  optimistic-load / conditional-branch sequence with unique labels.
- **CMP_GT** — uses the operand-swap trick: Ra > Rb ⟺ Rb < Ra.
- **RET** — copies C to A before RFC so the caller gets the return value.
- **SYSCALL** — inlined as native 8008 instructions (ADC, SBB, rotations,
  IN/OUT, parity).

### Output Format

```asm
    ORG 0x0000
_start:
    MVI  B, 0
    CAL  _fn_main
    HLT
_fn_main:
    MVI  C, 42
    MOV  A, C
    RFC
```

- `ORG 0x0000` at the top (4-space indent)
- Labels at column 0 with a colon suffix
- Instructions indented by 4 spaces
- Immediates in decimal; `0xFF` used for XRI constant

## Installation

```bash
pip install coding-adventures-ir-to-intel-8008-compiler
```

Or in development mode (from this directory):

```bash
uv pip install -e ../compiler-ir -e ../intel-8008-ir-validator -e ".[dev]"
```

## Usage

### Combined validate + generate

```python
from compiler_ir import IrProgram
from ir_to_intel_8008_compiler import IrToIntel8008Compiler

compiler = IrToIntel8008Compiler()
try:
    asm = compiler.compile(program)  # validates first
    print(asm)
except IrValidationError as e:
    print("Validation failed:", e)
```

### Validate only

```python
from ir_to_intel_8008_compiler import validate

errors = validate(program)
for err in errors:
    print(err.rule, err.message)
```

### Generate without validation

```python
from ir_to_intel_8008_compiler import generate_asm

asm = generate_asm(program)  # skips validation
```

### Low-level API

```python
from ir_to_intel_8008_compiler import CodeGenerator

gen = CodeGenerator()
asm = gen.generate(program)
```

## Running Tests

```bash
bash BUILD
```

Tests are in `tests/test_ir_to_intel_8008_compiler.py` and cover all 24 IR
opcodes, all 10 SYSCALL intrinsics, and the validate+compile facade.
