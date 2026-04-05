# TypeScript 5.0 (2023) Lexer

Tokenizes TypeScript 5.0 source code using the grammar-driven lexer.

## Overview

TypeScript 5.0, released March 2023, is the first version to adopt the standard
TC39 decorator proposal by default (without ``--experimentalDecorators``). It
targets the ES2022 baseline, adding class fields, private class members (``#name``),
and static initialization blocks.

This package is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``ts5.0.tokens`` grammar file from ``code/grammars/typescript/`` and
produces a stream of ``Token`` objects.

## Key Features (TS 5.0 / ES2022)

- **Standard decorators** — ``@decorator`` syntax using TC39 Stage-3 proposal
- **``accessor`` keyword** — auto-accessor class members for use with decorators
- **``satisfies`` operator** — type-check without widening the inferred type
- **``const`` type parameters** — preserve literal types in generic functions
- **Private class fields** — ``#name`` truly-private fields (runtime enforced)
- **Static initialization blocks** — ``static { ... }`` in class bodies
- **``using`` context keyword** — explicit resource management (TS 5.2)
- **Logical assignment** — ``||=``, ``&&=``, ``??=`` operators

## Usage

```python
from typescript_ts50_lexer import tokenize_ts50

tokens = tokenize_ts50('@decorator class Foo { accessor name = ""; }')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

## API

- ``tokenize_ts50(source: str) -> list[Token]`` — Tokenize source code, returns list of tokens ending with EOF.
- ``create_ts50_lexer(source: str) -> GrammarLexer`` — Create a lexer instance for advanced usage.

## Dependencies

- ``coding-adventures-grammar-tools`` — Parses ``.tokens`` files
- ``coding-adventures-lexer`` — Provides ``GrammarLexer`` engine
