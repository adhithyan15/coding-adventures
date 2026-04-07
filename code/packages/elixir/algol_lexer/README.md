# ALGOL 60 Lexer (Elixir)

Thin wrapper around the grammar-driven lexer engine for ALGOL 60 tokenization.

## What Is ALGOL 60?

ALGOL 60 (ALGOrithmic Language, 1960) was the most influential programming language
ever designed in terms of ideas-per-line-of-specification. It introduced:

- **Block structure** — `begin ... end` creates a nested lexical scope
- **Lexical scoping** — inner blocks see variables from enclosing blocks
- **Recursion** — the call stack was invented to support ALGOL 60
- **BNF grammar** — the first language formally specified using Backus-Naur Form
- **Free-format source** — whitespace between tokens is insignificant (unlike Fortran)

ALGOL 60 is the direct ancestor of Pascal, C, Ada, and Simula (the first
object-oriented language). Every language that uses `if`, `else`, `for`, `while`,
`begin`, or `end` is echoing ALGOL 60 vocabulary.

## Usage

```elixir
{:ok, tokens} = CodingAdventures.AlgolLexer.tokenize("begin integer x; x := 42 end")
# => [
#   %Token{type: "begin",       value: "begin"},
#   %Token{type: "integer",     value: "integer"},
#   %Token{type: "IDENT",       value: "x"},
#   %Token{type: "SEMICOLON",   value: ";"},
#   %Token{type: "IDENT",       value: "x"},
#   %Token{type: "ASSIGN",      value: ":="},
#   %Token{type: "INTEGER_LIT", value: "42"},
#   %Token{type: "end",         value: "end"},
#   %Token{type: "EOF",         value: ""}
# ]
```

## Token Inventory

| Token         | Lexeme(s)         | Notes                                    |
|---------------|-------------------|------------------------------------------|
| `ASSIGN`      | `:=`              | Assignment; `=` means equality           |
| `POWER`       | `**`              | Exponentiation (Fortran style)           |
| `CARET`       | `^`               | Exponentiation (alternative)             |
| `LEQ`         | `<=`              | Less-than-or-equal (≤)                   |
| `GEQ`         | `>=`              | Greater-than-or-equal (≥)               |
| `NEQ`         | `!=`              | Not-equal (≠)                            |
| `EQ`          | `=`               | Equality test                            |
| `LT` / `GT`   | `<` / `>`         | Comparison                               |
| `INTEGER_LIT` | `42`              | Plain decimal integer                    |
| `REAL_LIT`    | `3.14`, `1.5E3`   | Decimal or scientific notation           |
| `STRING_LIT`  | `'hello'`         | Single-quoted, no escapes                |
| `IDENT`       | `myVar`           | Letter + letters/digits; no underscore   |
| keywords      | `begin`, `end`, … | Reclassified from IDENT after full match |
| comments      | `comment … ;`     | Consumed silently, no token emitted      |

## How It Works

Reads `algol.tokens` from the shared `code/grammars/` directory and delegates
to `GrammarLexer.tokenize/2`. The grammar is cached via `persistent_term` for
fast repeated calls (file is read once per BEAM node lifetime).

## Dependencies

- `grammar_tools` — parses `.tokens` files into `TokenGrammar` structs
- `lexer` — grammar-driven tokenization engine
- `directed_graph` — transitive dependency of `grammar_tools`
