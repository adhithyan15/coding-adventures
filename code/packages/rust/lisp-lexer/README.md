# lisp-lexer

Tokenizes Lisp source code into a stream of tokens. This is the first stage of the Lisp compilation pipeline: source text goes in, a flat list of tokens comes out.

## Token Types

| Token     | Description                              | Examples            |
|-----------|------------------------------------------|---------------------|
| `Number`  | Integer literals, possibly negative      | `42`, `-7`, `0`     |
| `Symbol`  | Identifiers and operator names           | `define`, `+`, `car`|
| `String`  | Double-quoted string literals            | `"hello"`           |
| `LParen`  | Opening parenthesis                      | `(`                 |
| `RParen`  | Closing parenthesis                      | `)`                 |
| `Quote`   | Single quote (syntactic sugar)           | `'`                 |
| `Dot`     | Dot separator for dotted pairs           | `.`                 |
| `Eof`     | End of input                             | (implicit)          |

## Usage

```rust
use lisp_lexer::tokenize;

let tokens = tokenize("(+ 1 2)").unwrap();
// [LParen, Symbol("+"), Number(1), Number(2), RParen, Eof]
```

## How It Fits in the Stack

```
Source text --> [lisp-lexer] --> tokens --> [lisp-parser] --> AST --> ...
```

The lexer feeds into the parser, which builds an S-expression AST.
