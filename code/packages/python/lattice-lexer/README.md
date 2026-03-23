# Lattice Lexer (Python)

Thin wrapper around the grammar-driven lexer engine for Lattice tokenization.

## What This Package Does

Lattice is a CSS superset language (like Sass) that adds variables, mixins,
functions, and control flow to CSS. This package handles the first stage of
the Lattice compiler pipeline: turning raw Lattice source text into a stream
of tokens.

It loads the `lattice.tokens` grammar file from the shared `code/grammars/`
directory and feeds it to the generic `GrammarLexer`. The Lattice token
grammar extends CSS's ~39 token types with 5 new tokens for the Lattice
language features, plus `//` single-line comment support.

## Lattice-Specific Tokens

| Token | Example | Purpose |
|-------|---------|---------|
| VARIABLE | `$color`, `$font-size` | Sass-style variable references |
| EQUALS_EQUALS | `==` | Equality comparison in `@if` |
| NOT_EQUALS | `!=` | Inequality comparison in `@if` |
| GREATER_EQUALS | `>=` | Greater-or-equal comparison |
| LESS_EQUALS | `<=` | Less-or-equal comparison |

All standard CSS token types (IDENT, NUMBER, DIMENSION, HASH, STRING,
FUNCTION, AT_KEYWORD, etc.) are preserved unchanged.

## How It Fits in the Stack

```
lattice.tokens
    |
    v
grammar_tools          lexer
(parse_token_grammar)  (GrammarLexer)
    |                      |
    +----------+-----------+
               |
               v
    lattice_lexer.tokenize_lattice()
               |
               v
         list[Token]
               |
               v
      lattice-parser (next stage)
```

## Usage

```python
from lattice_lexer import tokenize_lattice

tokens = tokenize_lattice("$color: red;")
for token in tokens:
    print(f"{token.type}: {token.value}")
# VARIABLE: $color
# COLON: :
# IDENT: red
# SEMICOLON: ;
# EOF:
```

### Lower-Level Access

```python
from lattice_lexer import create_lattice_lexer

lexer = create_lattice_lexer("$size: 16px;")
tokens = lexer.tokenize()
```

## Installation

```bash
pip install coding-adventures-lattice-lexer
```

## Dependencies

- `coding-adventures-grammar-tools` -- parses the `.tokens` grammar file
- `coding-adventures-lexer` -- provides the `GrammarLexer` engine and `Token` type
