# Bytecode Compiler

**Layer 8 of the computing stack** — walks the AST and emits stack machine bytecode instructions.

## What this package does

Compiles abstract syntax trees into stack machine bytecode. Produces CodeObjects containing:

- **Instructions** — a sequence of stack machine bytecode operations
- **Constants pool** — literal values referenced by instructions
- **Names pool** — variable and function names referenced by instructions

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → [Bytecode Compiler] → VM
```

This package is used by the **virtual-machine** package to execute compiled bytecode.

## Installation

```bash
uv add coding-adventures-bytecode-compiler
```

## Usage

```python
from bytecode_compiler import compile

code_object = compile(ast)
code_object.instructions  # list of bytecode instructions
code_object.constants     # constants pool
code_object.names         # names pool
```

## Spec

See [08-bytecode-compiler.md](../../../specs/04-bytecode-compiler.md) for the full specification.
