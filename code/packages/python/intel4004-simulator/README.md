# Intel 4004 Simulator

**Layer 4d of the computing stack** — simulates the Intel 4004 (1971), the world's first commercial microprocessor. 4-bit, accumulator architecture.

## What this package does

Simulates the Intel 4004, the world's first commercial microprocessor. The 4004 uses a 4-bit accumulator architecture:

- Instruction decoding and execution (MVP: `LDM`, `XCH`, `ADD`, `HLT`)
- 4-bit register and accumulator management
- Historical microprocessor simulation
- Minimal instruction set faithful to the original design

## Where it fits

```
Logic Gates → Arithmetic → CPU → [Intel 4004 Simulator] → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **assembler** package to execute assembled Intel 4004 instructions.

## Installation

```bash
uv add coding-adventures-intel4004-simulator
```

## Usage

```python
from intel4004_simulator import decode, execute

# Load immediate value 5 into accumulator
instruction = decode(0xD5)  # LDM 5
execute(instruction)
```

## Spec

See [04d-intel4004-simulator.md](../../../specs/07d-intel4004-simulator.md) for the full specification.
