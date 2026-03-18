# CPU Simulator

**Layer 3 of the computing stack** — models the core of a CPU.

## What this package does

Models the essential components of a CPU core:

| Component | Description |
|-----------|-------------|
| Registers | General-purpose storage inside the CPU |
| Memory | Addressable byte-array for instructions and data |
| Program Counter | Tracks the address of the next instruction |
| Fetch-Decode-Execute | The core cycle that drives instruction processing |

## Where it fits

```
Logic Gates → Arithmetic → [CPU Simulator] → ARM → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **arm-simulator** package to model a complete ARM-based processor.

## Installation

```bash
uv add coding-adventures-cpu-simulator
```

## Usage

```python
from cpu_simulator import CPU, Memory

mem = Memory(size=256)
cpu = CPU(memory=mem)
cpu.run()
```

## Spec

See [03-cpu-simulator.md](../../../specs/03-cpu-simulator.md) for the full specification.
