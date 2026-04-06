# ALGOL 60 Lexer

A Ruby gem that tokenizes ALGOL 60 source text using the grammar-driven lexer engine.

## Overview

This gem is a thin wrapper around `coding_adventures_lexer`'s `GrammarLexer`. Instead of writing an ALGOL-specific tokenizer from scratch, it loads the `algol.tokens` grammar file and feeds it to the general-purpose lexer engine.

ALGOL 60 (ALGOrithmic Language, 1960) is one of the most historically significant programming languages ever created. It was the first language specified using BNF notation, introduced block structure, lexical scoping, recursion, and the call stack. Every modern imperative language — Pascal, C, Ada, Simula (the first OOP language), Java, Rust, Go — descends from ALGOL 60.

This demonstrates the core idea behind grammar-driven language tooling: the same engine can process any language, as long as you provide the right grammar file.

## How It Fits in the Stack

```
algol.tokens (grammar file)
       |
       v
grammar_tools (parses .tokens into TokenGrammar)
       |
       v
lexer (GrammarLexer uses TokenGrammar to tokenize)
       |
       v
algol_lexer (this gem -- thin wrapper providing ALGOL 60 API)
```

## Usage

```ruby
require "coding_adventures_algol_lexer"

tokens = CodingAdventures::AlgolLexer.tokenize("begin integer x; x := 42 end")
tokens.each { |t| puts t }
# Token(begin, "begin", 1:1)   -- keyword
# Token(integer, "integer", 1:7)
# Token(IDENT, "x", 1:15)
# Token(SEMICOLON, ";", 1:16)
# Token(IDENT, "x", 1:18)
# Token(ASSIGN, ":=", 1:20)
# Token(INTEGER_LIT, "42", 1:23)
# Token(end, "end", 1:26)
# Token(EOF, "", 1:29)
```

## Token Vocabulary

### Value tokens
| Token | Example | Notes |
|-------|---------|-------|
| `REAL_LIT` | `3.14`, `1.5E3`, `100E2` | Must precede `INTEGER_LIT` in the grammar |
| `INTEGER_LIT` | `0`, `42`, `1000` | Plain decimal digits |
| `STRING_LIT` | `'hello'`, `''` | Single-quoted; no escape sequences |
| `IDENT` | `x`, `sum`, `A1` | Letter followed by letters/digits; no underscore |

### Keywords (reclassified from IDENT)
`begin` `end` `if` `then` `else` `for` `do` `step` `until` `while` `goto`
`switch` `procedure` `own` `array` `label` `value`
`integer` `real` `boolean` `string`
`true` `false`
`not` `and` `or` `impl` `eqv` `div` `mod`
`comment` (triggers comment-skip; see below)

### Operators (multi-character first)
| Token | Symbol | Notes |
|-------|--------|-------|
| `ASSIGN` | `:=` | Assignment; must precede `COLON` |
| `POWER` | `**` | Exponentiation; must precede `STAR` |
| `LEQ` | `<=` | Less-or-equal; must precede `LT` |
| `GEQ` | `>=` | Greater-or-equal; must precede `GT` |
| `NEQ` | `!=` | Not-equal |
| `PLUS` | `+` | |
| `MINUS` | `-` | |
| `STAR` | `*` | |
| `SLASH` | `/` | |
| `CARET` | `^` | Alternative to `**` for exponentiation |
| `EQ` | `=` | Equality test (not assignment) |
| `LT` | `<` | |
| `GT` | `>` | |

### Delimiters
`LPAREN` `(` | `RPAREN` `)` | `LBRACKET` `[` | `RBRACKET` `]`
`SEMICOLON` `;` | `COMMA` `,` | `COLON` `:`

### Silently skipped
- Whitespace (spaces, tabs, CR, LF)
- Comments: `comment <any text up to the next semicolon>;`

## Comment Syntax

ALGOL 60 uses a distinctive comment syntax — the word `comment` followed by any text, terminated by a semicolon:

```algol
comment this is an ALGOL 60 comment;
x := 1;
comment another comment with no special chars;
```

The entire sequence (from `comment` through the `;`) is consumed silently by the lexer's skip rules.

## Key Differences from JSON Lexer

- **Keywords**: ALGOL has 30 keywords reclassified from IDENT after full-token match.
- **Identifier disambiguation**: `begin` → keyword, `beginning` → IDENT.
- **Operator priority**: `:=` before `:`, `**` before `*`, `<=` before `<`, etc.
- **Two numeric types**: `REAL_LIT` (with `.` or exponent) and `INTEGER_LIT`.
- **Comment skipping**: `comment...;` blocks are consumed by the grammar's skip rules.
- **Single-quoted strings**: `'hello'` — no escape sequences.

## Dependencies

- `coding_adventures_grammar_tools` — reads the `.tokens` grammar file
- `coding_adventures_lexer` — the grammar-driven lexer engine

## Development

```bash
bundle install
bundle exec rake test
```
