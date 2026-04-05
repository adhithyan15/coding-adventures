# TypeScript 3.0 (2018) Lexer

Tokenizes TypeScript 3.0 (2018) source code using the grammar-driven lexer.

## Overview

This package is a thin wrapper around the generic `GrammarLexer`. It loads
the `ts3.0.tokens` grammar file from `code/grammars/typescript/` and
produces a stream of `Token` objects.

TypeScript 3.0 (July 2018) shipped with an ES2018 baseline. The signature
addition was the `unknown` top type — a type-safe alternative to `any` that
forces you to narrow before use. Rest and spread support in tuple types was
also formalized.

## Usage

```python
from typescript_ts30_lexer import tokenize_ts30

tokens = tokenize_ts30('const x: unknown = 42;')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

## API

- `tokenize_ts30(source: str) -> list[Token]` — Tokenize source code, returns list of tokens ending with EOF.
- `create_ts30_lexer(source: str) -> GrammarLexer` — Create a lexer instance for advanced usage.

## TypeScript 3.0 Highlights

- `unknown` top type (safer `any`) — contextual keyword, lexed as NAME
- Rest elements in tuple types: `[string, ...number[]]`
- Spread in tuple types and generic spread expressions
- ES2018 baseline (async iteration, `Promise.finally`, rest properties)

## Dependencies

- `coding-adventures-grammar-tools` — Parses `.tokens` files
- `coding-adventures-lexer` — Provides `GrammarLexer` engine
