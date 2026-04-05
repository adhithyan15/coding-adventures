# TypeScript 4.0 (2020) Lexer

Tokenizes TypeScript 4.0 (2020) source code using the grammar-driven lexer.

## Overview

This package is a thin wrapper around the generic `GrammarLexer`. It loads
the `ts4.0.tokens` grammar file from `code/grammars/typescript/` and
produces a stream of `Token` objects.

TypeScript 4.0 (August 2020) shipped with an ES2020 baseline. The headline
additions were variadic tuple types, labeled tuple elements, template literal
types, and the short-circuit assignment operators (`&&=`, `||=`, `??=`).

## Usage

```python
from typescript_ts40_lexer import tokenize_ts40

tokens = tokenize_ts40('type Pair = [first: string, second: number];')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

## API

- `tokenize_ts40(source: str) -> list[Token]` — Tokenize source code, returns list of tokens ending with EOF.
- `create_ts40_lexer(source: str) -> GrammarLexer` — Create a lexer instance for advanced usage.

## TypeScript 4.0 Highlights

- Variadic tuple types: `[...T, ...U]` for generic tuple concatenation
- Labeled tuple elements: `[start: number, end: number]` for documentation
- Template literal types: `` `Hello, ${string}!` `` at the type level
- Short-circuit assignment: `&&=`, `||=`, `??=` (ES2021 operators)
- ES2020 baseline: optional chaining `?.`, nullish coalescing `??`, `BigInt`

## Dependencies

- `coding-adventures-grammar-tools` — Parses `.tokens` files
- `coding-adventures-lexer` — Provides `GrammarLexer` engine
