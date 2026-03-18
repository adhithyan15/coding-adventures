# WASM Simulator

**Layer 4c of the computing stack** — implements a minimal WebAssembly stack-based VM.

## What this package does

Simulates a minimal WebAssembly stack-based virtual machine. WebAssembly uses a stack-based architecture where instructions push and pop values from an operand stack:

- Stack-based instruction execution (MVP: `i32.const`, `i32.add`, `local.set`, `local.get`, `end`)
- Local variable management
- Operand stack simulation
- Clean, minimal instruction set

## Where it fits

```
Logic Gates → Arithmetic → CPU → [WASM Simulator] → Assembler → Lexer → Parser → Compiler → VM
```

This package is used by the **assembler** package to execute assembled WebAssembly instructions.

## Installation

```bash
uv add coding-adventures-wasm-simulator
```

## Usage

```python
from wasm_simulator import execute

# x = 1 + 2
instructions = [
    ("i32.const", 1),
    ("i32.const", 2),
    ("i32.add",),
    ("local.set", 0),
    ("end",),
]
execute(instructions)
```

## Spec

See [04c-wasm-simulator.md](../../../specs/04c-wasm-simulator.md) for the full specification.
