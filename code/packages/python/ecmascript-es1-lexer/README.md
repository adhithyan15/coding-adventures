# ECMAScript 1 (1997) Lexer

Tokenizes ECMAScript 1 (1997) JavaScript source code using the grammar-driven lexer.

## Overview

This package is a thin wrapper around the generic `GrammarLexer`. It loads
the `es1.tokens` grammar file from `code/grammars/ecmascript/` and
produces a stream of `Token` objects.

## Usage

```python
from ecmascript_es1_lexer import tokenize_es1

tokens = tokenize_es1('var x = 1 + 2;')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

## API

- `tokenize_es1(source: str) -> list[Token]` — Tokenize source code, returns list of tokens ending with EOF.
- `create_es1_lexer(source: str) -> GrammarLexer` — Create a lexer instance for advanced usage.

## Dependencies

- `coding-adventures-grammar-tools` — Parses `.tokens` files
- `coding-adventures-lexer` — Provides `GrammarLexer` engine
