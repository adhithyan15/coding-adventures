# intel-8008-assembler

A two-pass assembler that converts Intel 8008 assembly text into raw binary bytes.

This is the fourth stage of the Oct → Intel 8008 compiler pipeline:

```
Oct source (.oct)
    ↓  (oct-lexer, oct-parser, oct-type-checker)
Typed AST
    ↓  (oct-ir-compiler)
IrProgram
    ↓  (intel-8008-ir-validator)
Validated IrProgram
    ↓  (ir-to-intel-8008-compiler)
8008 Assembly text (.asm)    ← intel-8008-assembler reads THIS
    ↓  (this package)
Binary bytes                 → fed to intel-8008-packager
    ↓  (intel-8008-packager)
Intel HEX file (.hex)        → fed to intel8008-simulator
```

## What is the Intel 8008?

The Intel 8008 (1972) is the world's first single-chip 8-bit microprocessor.
It has a 14-bit address space (16 KB), seven registers (A, B, C, D, E, H, L),
a pseudo-register M (memory at H:L), and an 8-level hardware call stack.

## Two-Pass Algorithm

**Pass 1 — Symbol Collection**: Walk every line. Track a program counter (PC).
When a label is found, record `{label: PC}`. Advance PC by instruction size.

**Pass 2 — Code Emission**: Walk every line again. Encode each instruction using
the now-complete symbol table. Forward and backward references both work.

## Instruction Sizes

| Instruction form | Bytes |
|------------------|-------|
| Fixed opcodes (RFC/RET, HLT, RLC, RRC, RAL, RAR, Rxx conditional returns) | 1 |
| Register ops (MOV, ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP, INR, DCR, IN, OUT, RST) | 1 |
| Immediate ops (MVI r, d8; ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI) | 2 |
| Jump/call (JMP, CAL, JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP) | 3 |

## hi()/lo() Directives

The code generator loads 14-bit static variable addresses into H:L using:

```asm
    MVI  H, hi(counter)   ; H ← (addr >> 8) & 0x3F  (high 6 bits)
    MVI  L, lo(counter)   ; L ← addr & 0xFF           (low 8 bits)
```

The assembler resolves `hi(sym)` and `lo(sym)` in Pass 2 using the symbol table.

## Address Encoding (3-byte instructions)

```
JMP a14 → [0x44, a14 & 0xFF, (a14 >> 8) & 0x3F]
         byte1   low-8-bits   high-6-bits
```

## Usage

```python
from intel_8008_assembler import assemble, Intel8008Assembler, AssemblerError

# Convenience function
binary = assemble("""
    ORG 0x0000
_start:
    MVI  B, 0
    CAL  _fn_main
    HLT
_fn_main:
    MVI  D, 42
    MOV  A, D
    RFC
""")

# Class instance (reusable across multiple programs)
asm = Intel8008Assembler()
binary = asm.assemble(source_text)

# Error handling
try:
    assemble("    JMP  undefined_label")
except AssemblerError as e:
    print(f"Assembly failed: {e}")
```

## Register Encoding

| Register | Code | Role |
|----------|------|------|
| B | 0 | 1st local / 1st argument |
| C | 1 | 2nd local / return value |
| D | 2 | 3rd local / 1st syscall arg |
| E | 3 | 4th local / 2nd syscall arg |
| H | 4 | Memory address high byte |
| L | 5 | Memory address low byte |
| M | 6 | Memory at H:L (pseudo-register) |
| A | 7 | Accumulator (implicit in all ALU ops) |

## Tests

```bash
bash BUILD
```

Tests cover:
- Lexer tokenisation (labels, mnemonics, operands, hi/lo expressions)
- Every instruction encoding (all groups: 1-byte, 2-byte, 3-byte)
- Label resolution (forward and backward references)
- hi()/lo() directive evaluation
- ORG directive and padding
- Error cases (undefined labels, out-of-range values, unknown mnemonics)
