# ARM Simulator

**Layer 4 of the computing stack** — implements an ARMv7 instruction subset decoder and executor.

## What this package does

Decodes and executes a subset of the ARMv7 instruction set, bridging the gap between raw CPU operations and assembly language:

- Instruction decoding (data processing, load/store, branch)
- Register file management (R0-R15, CPSR)
- Condition code evaluation
- Barrel shifter operations
- Memory-mapped I/O simulation

## Where it fits

```
Logic Gates → Arithmetic → CPU → [ARM Simulator] → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **assembler** package to execute assembled ARM instructions.

## Installation

```bash
uv add coding-adventures-arm-simulator
```

## Usage

```python
from arm_simulator import decode, execute

instruction = decode(0xE2800001)  # ADD R0, R0, #1
execute(instruction)
```

## Spec

See [04-arm-simulator.md](../../../specs/07b-arm-simulator.md) for the full specification.
