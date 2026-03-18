# RISC-V Simulator

**Layer 4b of the computing stack** (alternative to ARM) — implements a minimal RISC-V RV32I instruction subset decoder and executor.

## What this package does

Decodes and executes a minimal subset of the RISC-V RV32I instruction set. RISC-V offers a cleaner encoding than ARM and is a fully open standard:

- Instruction decoding (just 3 instructions to start: `addi`, `add`, `ecall`)
- Register file management (x0-x31)
- Clean, regular instruction encoding
- Memory-mapped I/O simulation

## Where it fits

```
Logic Gates → Arithmetic → CPU → [RISC-V Simulator] → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **assembler** package to execute assembled RISC-V instructions.

## Installation

```bash
uv add coding-adventures-riscv-simulator
```

## Usage

```python
from riscv_simulator import decode, execute

# x = 1 + 2
instruction = decode(0x00100093)  # addi x1, x0, 1
execute(instruction)
instruction = decode(0x00200113)  # addi x2, x0, 2
execute(instruction)
instruction = decode(0x002081B3)  # add x3, x1, x2
execute(instruction)
```

## Spec

See [04b-riscv-simulator.md](../../../specs/04b-riscv-simulator.md) for the full specification.
