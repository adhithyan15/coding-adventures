# coding-adventures-python-lexer

A Python lexer for the coding-adventures project. This crate tokenizes Python source code (versions 2.7 through 3.12) using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads a versioned `python{version}.tokens` grammar file and feeds it to the generic `GrammarLexer`. Each grammar file defines all of that version's tokens -- keywords, operators, string literals, numbers, identifiers -- in a declarative format.

The lexer operates in **indentation mode**, meaning it tracks leading whitespace and emits synthetic `INDENT`, `DEDENT`, and `NEWLINE` tokens, following Python's significant-whitespace rules.

## Supported versions

| Version | Grammar file         | Notable features                              |
|---------|----------------------|-----------------------------------------------|
| 2.7     | python2.7.tokens     | Classic Python 2 syntax                       |
| 3.0     | python3.0.tokens     | Python 3 baseline (print function, etc.)      |
| 3.6     | python3.6.tokens     | f-strings, variable annotations               |
| 3.8     | python3.8.tokens     | Walrus operator `:=`, positional-only params  |
| 3.10    | python3.10.tokens    | `match`/`case` soft keywords (PEP 634)        |
| 3.12    | python3.12.tokens    | `type` soft keyword (PEP 695), f-string rework|

The default version is **3.12**.

## How it fits in the stack

```
python{version}.tokens  (grammar file)
       |
       v
grammar-tools           (parses .tokens into TokenGrammar)
       |
       v
lexer                   (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
python-lexer            (THIS CRATE: wires grammar + lexer together for Python)
       |
       v
python-parser           (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_python_lexer::{create_python_lexer, tokenize_python};

// Quick tokenization with default version (3.12)
let tokens = tokenize_python("x = 1 + 2\n", "3.12");

// Tokenize Python 2.7 code
let tokens = tokenize_python("print 'hello'\n", "2.7");

// Or get the lexer object for more control
let mut lexer = create_python_lexer("def f():\n    return 1\n", "3.12");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The Python lexer produces these token categories:

- **NAME** -- identifiers like `x`, `my_func`, `_private`
- **KEYWORD** -- reserved words: `def`, `if`, `else`, `for`, `return`, `class`, etc.
- **INT** / **FLOAT** / **COMPLEX** -- numeric literals
- **STRING** -- string literals (single/double/triple-quoted, raw, byte, f-string)
- **Operators** -- `+`, `-`, `*`, `/`, `//`, `**`, `==`, `!=`, `<=`, `>=`, `:=`, etc.
- **Delimiters** -- `(`, `)`, `[`, `]`, `{`, `}`, `,`, `:`, `.`, `;`, `@`
- **INDENT** / **DEDENT** / **NEWLINE** -- whitespace structure tokens
- **EOF** -- end of file

## Soft keywords (3.10+)

Python 3.10 introduced `match`, `case`, and `_` as soft keywords -- they act as keywords only inside match statements and remain valid identifiers everywhere else. Python 3.12 added `type` as a soft keyword (PEP 695). The grammar files declare these in a `soft_keywords:` section, and they are stored in the `TokenGrammar.soft_keywords` field for downstream parsers to use.
