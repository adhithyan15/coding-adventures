# TypeScript 5.8 (2025) Lexer

Tokenizes TypeScript 5.8 source code using the grammar-driven lexer.

## Overview

TypeScript 5.8, released February 2025, targets the ES2025 baseline. ES2025
standardizes three landmark features: TC39 decorators, import attributes
(``with { type: "json" }``), and explicit resource management (``using`` /
``await using``).

This package is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``ts5.8.tokens`` grammar file from ``code/grammars/typescript/`` and
produces a stream of ``Token`` objects.

## Key Features (TS 5.8 / ES2025)

- **HASHBANG token** — ``#!/usr/bin/env node`` at the start of scripts (ES2025)
- **``using`` context keyword** — explicit resource management (ES2025 standard)
- **``await using``** — async resource management with ``Symbol.asyncDispose``
- **Import attributes** — ``with { type: "json" }`` clause on imports
- **Standard decorators** — ``@`` decorator syntax (ES2025 standard)
- **Regex ``v`` flag** — Unicode Sets mode (ES2024, part of the ES2025 baseline)
- **``--erasableSyntaxOnly``** mode support (type-erasure-only syntax)
- **``export type *``** re-export syntax

## Usage

```python
from typescript_ts58_lexer import tokenize_ts58

tokens = tokenize_ts58('using conn = openConnection();')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

## API

- ``tokenize_ts58(source: str) -> list[Token]`` — Tokenize source code, returns list of tokens ending with EOF.
- ``create_ts58_lexer(source: str) -> GrammarLexer`` — Create a lexer instance for advanced usage.

## Dependencies

- ``coding-adventures-grammar-tools`` — Parses ``.tokens`` files
- ``coding-adventures-lexer`` — Provides ``GrammarLexer`` engine
