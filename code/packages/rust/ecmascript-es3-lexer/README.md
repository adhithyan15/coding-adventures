# coding-adventures-ecmascript-es3-lexer

An ECMAScript 3 (1999) lexer for the coding-adventures project. This crate tokenizes ES3 JavaScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `es3.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of ES3's tokens in a declarative format.

## What makes ES3 different from ES1

ES3 was the version that made JavaScript a real, complete language. Key additions:

- **`===` and `!==`** — strict equality operators (no type coercion)
- **`try`/`catch`/`finally`/`throw`** — structured error handling
- **`instanceof`** — prototype chain checking operator
- **Regex literals** — `/pattern/flags` formalized in the spec

## How it fits in the stack

```
es3.tokens          (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
ecmascript-es3-lexer (THIS CRATE: wires grammar + lexer together for ES3)
       |
       v
ecmascript-es3-parser (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_ecmascript_es3_lexer::{create_es3_lexer, tokenize_es3};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_es3("try { x(); } catch (e) { }");

// Or get the lexer object for more control
let mut lexer = create_es3_lexer("a === b;");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The ES3 lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`, `$dollar`
- **KEYWORD** — reserved words: `var`, `function`, `try`, `catch`, `finally`, `throw`, `instanceof`, etc.
- **NUMBER** — numeric literals (integers, floats, hex)
- **STRING** — string literals (single-quoted and double-quoted)
- **REGEX** — regular expression literals (`/pattern/flags`)
- **Operators** — `+`, `-`, `*`, `/`, `===`, `!==`, `==`, `!=`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file
