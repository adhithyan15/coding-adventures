# intel-4004-assembler

A two-pass assembler that converts Intel 4004 assembly text into binary bytes.

## Overview

This is PR 10 in the Nib language → Intel 4004 compiler pipeline. It sits after
the `ir-to-intel-4004-compiler` package (which generates assembly text from an `IrProgram`) and
produces raw binary bytes suitable for loading into the `intel4004-simulator`.

### Pipeline position

```
Nib source
    ↓  (nib-lexer, nib-parser)
AST
    ↓  (nib-type-checker)
Typed AST
    ↓  (nib-ir-compiler)
IrProgram
    ↓  (ir-optimizer)
Optimised IrProgram
    ↓  (ir-to-intel-4004-compiler)
Assembly text             ← intel-4004-assembler reads this
    ↓  (this package)
Binary bytes              → fed to intel4004-simulator
```

## Installation

```bash
pip install coding-adventures-intel-4004-assembler
```

## Usage

```python
from intel_4004_assembler import assemble, Intel4004Assembler, AssemblerError

# Convenience function
binary = assemble("""
    ORG 0x000
_start:
    LDM 5
    XCH R2
    HLT
""")
print(binary.hex())  # "d5b201"

# Reusable class instance
asm = Intel4004Assembler()
binary = asm.assemble(source_text)

# Error handling
try:
    assemble("    JUN undefined_label")
except AssemblerError as e:
    print(f"Assembly failed: {e}")
```

## Input Format

```asm
    ORG 0x000           ; set program counter to 0
_start:                 ; label definition
    LDM 5               ; load immediate 5 → ACC
    XCH R2              ; swap ACC ↔ R2
    NOP
loop_0_start:
    LD R2
    JCN 0x4, loop_0_end ; jump if zero
    ADD_IMM R2, R2, 1   ; pseudo-instruction: R2 + 1 → ACC
    JUN loop_0_start
loop_0_end:
    JUN $               ; self-loop (halt equivalent)
```

Rules:
- Lines starting with whitespace are instructions
- Lines ending with `:` are labels (indentation optional)
- `;` starts a comment (rest of line ignored)
- `ORG addr` sets the address counter
- `$` in a JUN operand means "current address" (self-loop)
- Register names: `R0`–`R15`, pairs `P0`–`P7`
- `HLT` is a simulator-only opcode (maps to `0x01`)

## Two-Pass Algorithm

**Pass 1** — builds the symbol table by scanning labels and tracking PC.  
**Pass 2** — encodes each instruction using the completed symbol table.

Forward references are fully supported — a `JUN loop_end` before `loop_end:` works correctly.

## Supported Instructions

All 46 Intel 4004 instructions are supported, plus the simulator-only `HLT` and the `ADD_IMM` pseudo-instruction.

See `src/intel_4004_assembler/encoder.py` for the complete opcode table with encoding details.

## Development

```bash
uv run pytest
uv run ruff check src/
```
