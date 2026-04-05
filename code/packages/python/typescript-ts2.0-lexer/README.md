# TypeScript 2.0 (September 2016) Lexer

Tokenizes TypeScript 2.0 source code using the grammar-driven lexer.

## What Was TypeScript 2.0?

TypeScript 2.0 was released in September 2016. It was a major release that
upgraded the JavaScript baseline from ECMAScript 5 to ECMAScript 2015 (ES6),
bringing many new syntactic constructs:

**Type System Additions:**
- **Non-nullable types** — strict null checks (`--strictNullChecks`)
- **`never` type** — for functions that never return (always throw or infinite loop)
- **`object` type** — represents non-primitive types
- **Tagged union types** — discriminated unions with `never` in exhaustiveness checks

**ES2015 Baseline (inherited from the ES6 upgrade):**
- `let` / `const` — block-scoped declarations
- Template literals — `` `Hello ${name}` ``
- Arrow functions — `(x) => x + 1`
- Classes — `class Foo extends Bar implements IBaz`
- ES2015 modules — `import { Foo } from "./foo"`
- Destructuring — `const { x, y } = obj`
- Default parameters — `function foo(x = 1)`
- Rest/spread — `function foo(...args)`, `[...arr]`
- `for...of` loops — `for (const x of arr)`

## Overview

This package is a thin wrapper around the generic `GrammarLexer`. It loads
the `ts2.0.tokens` grammar file from `code/grammars/typescript/` and
produces a stream of `Token` objects.

## Usage

```python
from typescript_ts20_lexer import tokenize_ts20

tokens = tokenize_ts20('const x: string | never = "hello";')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
```

Using the factory function for advanced usage:

```python
from typescript_ts20_lexer import create_ts20_lexer

lexer = create_ts20_lexer('import { Foo } from "./foo";')
tokens = lexer.tokenize()
```

## API

- `tokenize_ts20(source: str) -> list[Token]` — Tokenize source code, returns list of tokens ending with EOF.
- `create_ts20_lexer(source: str) -> GrammarLexer` — Create a lexer instance for advanced usage.

## New Tokens vs TS 1.0

TS 2.0 inherits all TS 1.0 tokens and adds ES2015 tokens:

| Token | Symbol | Example |
|-------|--------|---------|
| `BACKTICK` | `` ` `` | `` `hello` `` |
| `TEMPLATE_MIDDLE` | `}...${` | interpolation |
| `DOUBLE_STAR` | `**` | exponentiation |

Context keywords added in TS 2.0: `never`, `object`, `readonly`, `is`,
`infer`, `unique`, `global`.

## Dependencies

- `coding-adventures-grammar-tools` — Parses `.tokens` files
- `coding-adventures-lexer` — Provides `GrammarLexer` engine
