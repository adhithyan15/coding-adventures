# coding-adventures-ecmascript-es1-lexer

An ECMAScript 1 (1997) lexer for the coding-adventures project. This crate tokenizes ES1 JavaScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `es1.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of ES1's tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## What makes ES1 different

ES1 was the very first ECMAScript standard (1997). It has notable limitations compared to later versions:

- **No `===` or `!==`** — only abstract equality (`==` and `!=`) with type coercion
- **No `try`/`catch`/`finally`/`throw`** — no structured error handling
- **No regex literals** — regular expressions were implementation-defined
- **No `instanceof`** — this operator was added in ES3

## How it fits in the stack

```
es1.tokens          (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
ecmascript-es1-lexer (THIS CRATE: wires grammar + lexer together for ES1)
       |
       v
ecmascript-es1-parser (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_ecmascript_es1_lexer::{create_es1_lexer, tokenize_es1};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_es1("var x = 42;");

// Or get the lexer object for more control
let mut lexer = create_es1_lexer("function add(a, b) { return a + b; }");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The ES1 lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`, `$dollar`
- **KEYWORD** — reserved words: `var`, `function`, `return`, `if`, `else`, `for`, `while`, etc.
- **NUMBER** — numeric literals (integers, floats, hex)
- **STRING** — string literals (single-quoted and double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `==`, `!=`, `&&`, `||`, etc. (no `===` or `!==`)
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file
