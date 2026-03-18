# Parser

**Layer 7 of the computing stack** — builds abstract syntax trees from token streams.

> **Note:** The Python module name is `lang_parser` (not `parser`) to avoid conflict with Python's built-in `parser` module.

## What this package does

Builds abstract syntax trees (ASTs) from token streams using recursive descent parsing:

| Feature | Description |
|---------|-------------|
| Recursive descent | Top-down parsing strategy, one function per grammar rule |
| Operator precedence | Correctly handles precedence and associativity |
| Expression parsing | Parses arithmetic, boolean, and comparison expressions |
| Statement parsing | Parses assignments, conditionals, loops, and function definitions |
| AST construction | Produces a tree representation suitable for compilation |

## Where it fits

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → [Parser] → Compiler → VM
```

This package is used by the **bytecode-compiler** package to generate bytecode from the AST.

## Installation

```bash
uv add coding-adventures-parser
```

## Usage

```python
from lang_parser import parse

tokens = [...]  # token stream from lexer
ast = parse(tokens)
```

## Spec

See [07-parser.md](../../../specs/07-parser.md) for the full specification.
