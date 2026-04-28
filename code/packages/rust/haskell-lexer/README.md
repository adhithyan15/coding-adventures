# coding-adventures-haskell-lexer

A Haskell lexer for the coding-adventures project. This crate tokenizes Haskell source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the appropriate `haskell{version}.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of Haskell's tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
haskell{version}.tokens  (grammar file)
       |
       v
grammar-tools         (parses .tokens into TokenGrammar)
       |
       v
lexer                 (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
haskell-lexer            (THIS CRATE: wires grammar + lexer together for Haskell)
       |
       v
haskell-parser           (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_haskell_lexer::{create_haskell_lexer, tokenize_haskell};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_haskell("class Hello { }", "21").unwrap();

// Or get the lexer object for more control
let mut lexer = create_haskell_lexer("public static void main(String[] args) { }", "21").unwrap();
let tokens = lexer.tokenize().expect("tokenization failed");

// Use a specific Haskell version
let tokens_8 = tokenize_haskell("int x = 42;", "8").unwrap();
```

## Token types

The Haskell lexer produces these token categories:

- **NAME** — identifiers like `x`, `MyClass`, `_private`
- **KEYWORD** — reserved words: `class`, `public`, `static`, `void`, `int`, `if`, `else`, `return`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file

## Supported Haskell versions

| Version | Grammar file |
|---------|-------------|
| `"1.0"` | `grammars/haskell/haskell1.0.tokens` |
| `"1.1"` | `grammars/haskell/haskell1.1.tokens` |
| `"1.4"` | `grammars/haskell/haskell1.4.tokens` |
| `"5"` | `grammars/haskell/haskell5.tokens` |
| `"7"` | `grammars/haskell/haskell7.tokens` |
| `"8"` | `grammars/haskell/haskell8.tokens` |
| `"10"` | `grammars/haskell/haskell10.tokens` |
| `"14"` | `grammars/haskell/haskell14.tokens` |
| `"17"` | `grammars/haskell/haskell17.tokens` |
| `"21"` (default) | `grammars/haskell/haskell21.tokens` |
