# TypeScript 1.0 (April 2014) Lexer

Tokenizes TypeScript 1.0 source code using the grammar-driven lexer.

## What Was TypeScript 1.0?

TypeScript 1.0 was the first public release of TypeScript, announced by
Microsoft at the Build developer conference in April 2014. It was a strict
superset of ECMAScript 5, meaning every valid JavaScript program was also
valid TypeScript.

TypeScript 1.0 added a static type system to JavaScript:

- **Type annotations** — `var x: number = 1;`
- **Interfaces** — `interface Foo { x: string; }`
- **Classes** — `class Animal { name: string; }`
- **Enums** — `enum Color { Red, Green, Blue }`
- **Generics** — `Array<string>`, `Map<string, number>`
- **Type aliases** — `type Alias = string;`
- **Namespaces** — `namespace MyNS { }`
- **Ambient declarations** — `declare var x: number;`
- **Type assertions** — `<string>x` and `x as string`
- **Decorators** (experimental) — `@Component`
- **Non-null assertion** — `x!`

## Overview

This package is a thin wrapper around the generic `GrammarLexer`. It loads
the `ts1.0.tokens` grammar file from `code/grammars/typescript/` and
produces a stream of `Token` objects.

## Usage

```python
from typescript_ts10_lexer import tokenize_ts10

tokens = tokenize_ts10('var x: number = 1;')
for token in tokens:
    print(f"{token.type.name}: {token.value!r}")
# KEYWORD: 'var'
# NAME: 'x'
# COLON: ':'
# NAME: 'number'
# EQUALS: '='
# NUMBER: '1'
# SEMICOLON: ';'
# EOF: ''
```

Using the factory function for advanced usage:

```python
from typescript_ts10_lexer import create_ts10_lexer

lexer = create_ts10_lexer('interface Foo { x: string; }')
tokens = lexer.tokenize()
```

## API

- `tokenize_ts10(source: str) -> list[Token]` — Tokenize source code, returns list of tokens ending with EOF.
- `create_ts10_lexer(source: str) -> GrammarLexer` — Create a lexer instance for advanced usage.

## New Tokens vs ES5

| Token | Symbol | Example |
|-------|--------|---------|
| `AT` | `@` | `@Component` |
| `COLON` | `:` | `x: number` |
| `LESS_THAN` | `<` | `Array<string>` |
| `GREATER_THAN` | `>` | `Array<string>` |
| `QUESTION_MARK` | `?` | `x?: string` |
| `EXCLAMATION` | `!` | `x!` |
| `FAT_ARROW` | `=>` | `(x: string) => void` |

## Context Keywords

Many TypeScript keywords are emitted as `NAME` tokens because they are
context-sensitive. Only the parser can resolve their meaning:

- `interface`, `type`, `namespace`, `declare`, `abstract`, `readonly`
- `from`, `of`, `as`, `is`, `keyof`, `typeof` (in type positions)

## Dependencies

- `coding-adventures-grammar-tools` — Parses `.tokens` files
- `coding-adventures-lexer` — Provides `GrammarLexer` engine
