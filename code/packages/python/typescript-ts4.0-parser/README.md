# TypeScript 4.0 (2020) Parser

Parses TypeScript 4.0 (2020) source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `ts4.0.grammar` file from `code/grammars/typescript/` and produces
`ASTNode` trees from tokenized source code.

TypeScript 4.0 (August 2020) introduced variadic tuple types, labeled tuple
elements, template literal types, and the short-circuit assignment operators
(`&&=`, `||=`, `??=`) on an ES2020 baseline.

## Usage

```python
from typescript_ts40_parser import parse_ts40

ast = parse_ts40('type Pair = [first: string, second: number];')
print(ast.rule_name)  # "program"
```

## API

- `parse_ts40(source: str) -> ASTNode` — Parse source code, returns the root AST node.
- `create_ts40_parser(source: str) -> GrammarParser` — Create a parser instance for advanced usage.

## TypeScript 4.0 Grammar Highlights

- Variadic tuple types: `[...T, ...U]` for generic tuple concatenation
- Labeled tuple elements: `[start: number, end: number]` (documentation labels)
- Template literal types: `` `Hello, ${string}!` `` at the type level
- Short-circuit assignment: `&&=`, `||=`, `??=`
- ES2020 baseline: optional chaining `?.`, nullish coalescing `??`, `BigInt`
- `unknown` type (from TS 3.0) in type positions

## Dependencies

- `coding-adventures-typescript-ts4.0-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
