# coding-adventures-toml-lexer (Lua)

A TOML lexer that tokenizes TOML source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `toml.tokens` grammar file.

## What it does

Given the input:

```toml
[server]
host = "localhost"
port = 8080
debug = true
```

The lexer produces a flat stream of typed tokens. Horizontal whitespace (spaces, tabs) and TOML comments (`#...`) are silently consumed. Newlines are significant in TOML and are emitted as tokens so a downstream parser can use them as statement terminators.

## Token types

| Token type         | Example match                      |
|--------------------|------------------------------------|
| BARE_KEY           | `my-key`, `server`, `_port`        |
| BASIC_STRING       | `"hello"`, `"a\nb"`, `"\u0041"`    |
| LITERAL_STRING     | `'C:\path'`                        |
| ML_BASIC_STRING    | `"""multi\nline"""`                |
| ML_LITERAL_STRING  | `'''multi\nline'''`                |
| INTEGER            | `42`, `-17`, `0xFF`, `0o755`, `0b1010`, `1_000` |
| FLOAT              | `3.14`, `-0.5`, `5e22`, `inf`, `nan` |
| TRUE               | `true`                             |
| FALSE              | `false`                            |
| OFFSET_DATETIME    | `1979-05-27T07:32:00Z`             |
| LOCAL_DATETIME     | `1979-05-27T07:32:00`              |
| LOCAL_DATE         | `1979-05-27`                       |
| LOCAL_TIME         | `07:32:00`, `07:32:00.999`         |
| EQUALS             | `=`                                |
| DOT                | `.`                                |
| COMMA              | `,`                                |
| LBRACKET           | `[`                                |
| RBRACKET           | `]`                                |
| LBRACE             | `{`                                |
| RBRACE             | `}`                                |
| EOF                | (end of input)                     |

## Usage

```lua
local toml_lexer = require("coding_adventures.toml_lexer")

local tokens = toml_lexer.tokenize('key = "value"')
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
toml.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
toml_lexer  ← you are here
    ↓  feeds
toml_parser  (future)
```

## TOML-specific notes

**Newlines are significant** — TOML key-value pairs are terminated by newlines. The `toml.tokens` grammar skips only horizontal whitespace (spaces and tabs). The lexer emits a NEWLINE token for each line ending.

**Pattern ordering** — The grammar carefully orders more-specific patterns before less-specific ones:
- Multi-line strings (`"""`, `'''`) before single-line strings
- Date/time patterns before bare keys and integers
- Floats before integers (`3.14` would otherwise match as `INTEGER(3) DOT INTEGER(14)`)
- `true`/`false` before BARE_KEY

**Aliases** — Multiple pattern names map to a single token type:
- `FLOAT_SPECIAL`, `FLOAT_EXP`, `FLOAT_DEC` all emit as `FLOAT`
- `HEX_INTEGER`, `OCT_INTEGER`, `BIN_INTEGER` all emit as `INTEGER`

## Dependencies

- `coding-adventures-grammar-tools` — parses `toml.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
