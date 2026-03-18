# Virtual Machine

**Layer 9 of the computing stack** — the top layer, a stack-based bytecode interpreter.

## What this package does

Implements the eval loop that executes compiled bytecode programs. This is what CPython, YARV, and the JVM do:

| Component | Description |
|-----------|-------------|
| Eval Loop | Fetch-decode-execute cycle over bytecode instructions |
| Stack | Operand stack for intermediate values |
| Frames | Call frames for function invocation |
| Globals | Global variable storage |

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → [VM]
```

This package is the **top layer** that ties everything together, consuming bytecode produced by the **bytecode-compiler** package.

## Installation

```bash
uv add coding-adventures-virtual-machine
```

## Usage

```python
from virtual_machine import VM

vm = VM()
vm.run(bytecode)
```

## Spec

See [09-virtual-machine.md](../../../specs/05-virtual-machine.md) for the full specification.
