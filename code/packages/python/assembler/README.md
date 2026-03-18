# Assembler

**Layer 5 of the computing stack** — translates human-readable assembly language to binary machine code.

## What this package does

Implements a two-pass assembler that converts assembly source into executable machine code:

| Feature | Description |
|---------|-------------|
| Two-pass assembly | First pass collects labels, second pass emits binary |
| Labels | Named locations in code for branch targets and data |
| Symbol tables | Maps label names to memory addresses |
| Instruction encoding | Translates mnemonics and operands to binary machine code |

Connects the software layers (lexer, parser, compiler) to the hardware layers (CPU, arithmetic, logic gates).

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → [Assembler] → Lexer → Parser → Compiler → VM
```

This package depends on the **ARM simulator** package and is used by the **lexer** to bridge high-level code down to machine instructions.

## Installation

```bash
uv add coding-adventures-assembler
```

## Usage

```python
from assembler import assemble

binary = assemble("MOV R0, #42\nADD R1, R0, #1\n")
```

## Spec

See [05-assembler.md](../../../specs/06-assembler.md) for the full specification.
