# TypeScript 3.0 (2018) Parser

Parses TypeScript 3.0 (2018) source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `ts3.0.grammar` file from `code/grammars/typescript/` and produces
`ASTNode` trees from tokenized source code.

TypeScript 3.0 (July 2018) introduced the `unknown` top type — a type-safe
replacement for `any` that requires narrowing before use — as well as rest
and spread support in tuple types.

## Usage

```python
from typescript_ts30_parser import parse_ts30

ast = parse_ts30('const x: unknown = 42;')
print(ast.rule_name)  # "program"
```

## API

- `parse_ts30(source: str) -> ASTNode` — Parse source code, returns the root AST node.
- `create_ts30_parser(source: str) -> GrammarParser` — Create a parser instance for advanced usage.

## TypeScript 3.0 Grammar Highlights

- `unknown` type in type annotation positions
- Rest elements in tuple type expressions: `[string, ...number[]]`
- Generic type parameters and constraints
- Interface and class declarations
- ES2018 baseline: async iteration, rest/spread in object literals

## Dependencies

- `coding-adventures-typescript-ts3.0-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
