# coding-adventures-typescript-lexer

A TypeScript lexer for the coding-adventures project. This crate tokenizes TypeScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `typescript.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of TypeScript's tokens — keywords (including TypeScript-specific ones like `interface`, `type`, `enum`), identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
typescript.tokens   (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
typescript-lexer    (THIS CRATE: wires grammar + lexer together for TypeScript)
       |
       v
typescript-parser   (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_typescript_lexer::{create_typescript_lexer, tokenize_typescript};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_typescript("let x: number = 42;");

// Or get the lexer object for more control
let mut lexer = create_typescript_lexer("function add(a: number, b: number): number { return a + b; }");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The TypeScript lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`
- **KEYWORD** — reserved words: `var`, `let`, `const`, `function`, `return`, `if`, `else`, `interface`, `type`, `enum`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (single-quoted and double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `===`, `!==`, `=>`, `&&`, `||`, `<`, `>`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file
