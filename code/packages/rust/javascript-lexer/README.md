# coding-adventures-javascript-lexer

A JavaScript lexer for the coding-adventures project. This crate tokenizes JavaScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `javascript.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of JavaScript's tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
javascript.tokens   (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
javascript-lexer    (THIS CRATE: wires grammar + lexer together for JavaScript)
       |
       v
javascript-parser   (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_javascript_lexer::{create_javascript_lexer, tokenize_javascript};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_javascript("var x = 42;");

// Or get the lexer object for more control
let mut lexer = create_javascript_lexer("function add(a, b) { return a + b; }");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The JavaScript lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`
- **KEYWORD** — reserved words: `var`, `let`, `const`, `function`, `return`, `if`, `else`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (single-quoted and double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `===`, `!==`, `=>`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file
