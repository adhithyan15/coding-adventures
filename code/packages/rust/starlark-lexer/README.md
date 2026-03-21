# coding-adventures-starlark-lexer

A Starlark lexer for the coding-adventures project. This crate tokenizes Starlark source code (the configuration language used by Bazel, Buck, and other build systems) using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `starlark.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of Starlark's tokens — keywords, operators, string literals, numbers, identifiers — in a declarative format.

The lexer operates in **indentation mode**, meaning it tracks leading whitespace and emits synthetic `INDENT`, `DEDENT`, and `NEWLINE` tokens, following Python/Starlark's significant-whitespace rules.

## How it fits in the stack

```
starlark.tokens  (grammar file)
       |
       v
grammar-tools    (parses .tokens into TokenGrammar)
       |
       v
lexer            (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
starlark-lexer   (THIS CRATE: wires grammar + lexer together for Starlark)
       |
       v
starlark-parser  (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_starlark_lexer::{create_starlark_lexer, tokenize_starlark};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_starlark("x = 1 + 2");

// Or get the lexer object for more control
let mut lexer = create_starlark_lexer("def f():\n    return 1\n");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The Starlark lexer produces these token categories:

- **NAME** — identifiers like `x`, `my_func`, `_private`
- **KEYWORD** — reserved words: `def`, `if`, `else`, `for`, `return`, etc.
- **INT** / **FLOAT** — numeric literals
- **STRING** — string literals (single/double/triple-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `//`, `**`, `==`, `!=`, `<=`, `>=`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `:`, `.`, `;`
- **INDENT** / **DEDENT** / **NEWLINE** — whitespace structure tokens
- **EOF** — end of file

Reserved keywords (`class`, `import`, `while`, etc.) cause a lexer error if encountered, since Starlark deliberately excludes these Python features.
