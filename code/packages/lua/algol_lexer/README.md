# coding-adventures-algol-lexer (Lua)

An ALGOL 60 lexer that tokenizes ALGOL 60 source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `algol.tokens` grammar file.

## What it does

Given the input `begin integer x; x := 42 end`, the lexer produces:

| # | Type        | Value     |
|---|-------------|-----------|
| 1 | BEGIN       | `begin`   |
| 2 | INTEGER     | `integer` |
| 3 | IDENT       | `x`       |
| 4 | SEMICOLON   | `;`       |
| 5 | IDENT       | `x`       |
| 6 | ASSIGN      | `:=`      |
| 7 | INTEGER_LIT | `42`      |
| 8 | END         | `end`     |
| 9 | EOF         |           |

Whitespace and comments (`comment ... ;`) are silently consumed — the parser never sees them.

## Token types

### Value tokens

| Token type  | Example match                                      |
|-------------|----------------------------------------------------|
| INTEGER_LIT | `0`, `42`, `1000`                                  |
| REAL_LIT    | `3.14`, `1.5E3`, `1.5E-3`, `100E2`                |
| STRING_LIT  | `'hello'`, `''`                                    |
| IDENT       | `x`, `sum`, `A1`, `myVariable`                     |

### Keywords

| Token type  | Source text (case-insensitive)      |
|-------------|-------------------------------------|
| BEGIN       | `begin`                             |
| END         | `end`                               |
| IF          | `if`                                |
| THEN        | `then`                              |
| ELSE        | `else`                              |
| FOR         | `for`                               |
| DO          | `do`                                |
| STEP        | `step`                              |
| UNTIL       | `until`                             |
| WHILE       | `while`                             |
| GOTO        | `goto`                              |
| SWITCH      | `switch`                            |
| PROCEDURE   | `procedure`                         |
| OWN         | `own`                               |
| ARRAY       | `array`                             |
| LABEL       | `label`                             |
| VALUE       | `value`                             |
| INTEGER     | `integer`                           |
| REAL        | `real`                              |
| BOOLEAN     | `boolean`                           |
| STRING      | `string`                            |
| TRUE        | `true`                              |
| FALSE       | `false`                             |
| NOT         | `not`                               |
| AND         | `and`                               |
| OR          | `or`                                |
| IMPL        | `impl`                              |
| EQV         | `eqv`                               |
| DIV         | `div`                               |
| MOD         | `mod`                               |

### Operators and delimiters

| Token type | Source text |
|------------|-------------|
| ASSIGN     | `:=`        |
| POWER      | `**`        |
| LEQ        | `<=`        |
| GEQ        | `>=`        |
| NEQ        | `!=`        |
| PLUS       | `+`         |
| MINUS      | `-`         |
| STAR       | `*`         |
| SLASH      | `/`         |
| CARET      | `^`         |
| EQ         | `=`         |
| LT         | `<`         |
| GT         | `>`         |
| LPAREN     | `(`         |
| RPAREN     | `)`         |
| LBRACKET   | `[`         |
| RBRACKET   | `]`         |
| SEMICOLON  | `;`         |
| COMMA      | `,`         |
| COLON      | `:`         |
| EOF        | (end of input) |

## Usage

```lua
local algol_lexer = require("coding_adventures.algol_lexer")

local tokens = algol_lexer.tokenize("begin integer x; x := 42 end")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## Comments

ALGOL 60 comments start with the keyword `comment` and end at the next semicolon:

```algol
comment this is a comment;
x := 1
```

The lexer silently discards everything from `comment` through the closing `;`.

## Keywords are case-insensitive

`BEGIN`, `Begin`, and `begin` all produce a `BEGIN` token. The `value` field preserves the original source text; the `type` field is always uppercase.

## Keyword boundary enforcement

The token `beginning` is classified as an `IDENT`, not `BEGIN` followed by something. Only a complete word match (not a prefix) promotes an identifier to a keyword.

## How it fits in the stack

```
algol.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
algol_lexer  ← you are here
    ↓  feeds
algol_parser
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `algol.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```

## Version

0.1.0
