# coding-adventures-css-lexer

A CSS lexer for the coding-adventures project. This crate tokenizes CSS source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `css.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of CSS's tokens — identifiers, at-keywords, hash tokens, dimensions, numbers, strings, operators, and delimiters — in a declarative format.

CSS tokenization is notably more complex than JSON because of compound tokens (`10px` is one DIMENSION token, not NUMBER + IDENT), context-dependent disambiguation (`#fff` as color vs `#header` as ID — both are HASH tokens), and diverse token types (at-keywords, function tokens, URL tokens, unicode ranges).

## How it fits in the stack

```
css.tokens       (grammar file)
       |
       v
grammar-tools    (parses .tokens into TokenGrammar)
       |
       v
lexer            (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
css-lexer        (THIS CRATE: wires grammar + lexer together for CSS)
       |
       v
css-parser       (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_css_lexer::{create_css_lexer, tokenize_css};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_css("body { color: red; }");

// Or get the lexer object for more control
let mut lexer = create_css_lexer("h1 { font-size: 16px; }");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The CSS lexer produces these token categories:

- **IDENT** — identifiers like `body`, `color`, `font-size`
- **AT_KEYWORD** — at-rules like `@media`, `@import`, `@keyframes`
- **HASH** — hash tokens for colors (`#ff0000`) and IDs (`#main`)
- **STRING** — quoted strings (single or double)
- **NUMBER** — plain numbers like `0`, `3.14`
- **DIMENSION** — numbers with units like `16px`, `1.5em`, `100%`
- **PERCENTAGE** — percentage values like `50%`
- **Delimiters** — `{`, `}`, `(`, `)`, `[`, `]`, `:`, `;`, `,`
- **EOF** — end of file
