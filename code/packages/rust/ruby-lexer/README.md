# coding-adventures-ruby-lexer

A Ruby lexer for the coding-adventures project. This crate tokenizes Ruby source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `ruby.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of Ruby's tokens — keywords, identifiers, numbers, strings, symbols, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
ruby.tokens      (grammar file)
       |
       v
grammar-tools    (parses .tokens into TokenGrammar)
       |
       v
lexer            (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
ruby-lexer       (THIS CRATE: wires grammar + lexer together for Ruby)
       |
       v
ruby-parser      (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_ruby_lexer::{create_ruby_lexer, tokenize_ruby};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_ruby("x = 1 + 2");

// Or get the lexer object for more control
let mut lexer = create_ruby_lexer("def greet(name)\n  puts name\nend");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The Ruby lexer produces these token categories:

- **NAME** — identifiers like `x`, `my_method`, `_private`
- **KEYWORD** — reserved words: `def`, `end`, `if`, `else`, `class`, `module`, `return`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (single-quoted and double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `.`, `:`, `;`
- **EOF** — end of file
