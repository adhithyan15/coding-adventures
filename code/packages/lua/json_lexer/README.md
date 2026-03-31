# coding-adventures-json-lexer (Lua)

A JSON lexer that tokenizes JSON source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `json.tokens` grammar file.

## What it does

Given the input `{"key": 42, "ok": true}`, the lexer produces:

| # | Type      | Value   |
|---|-----------|---------|
| 1 | LBRACE    | `{`     |
| 2 | STRING    | `"key"` |
| 3 | COLON     | `:`     |
| 4 | NUMBER    | `42`    |
| 5 | COMMA     | `,`     |
| 6 | STRING    | `"ok"`  |
| 7 | COLON     | `:`     |
| 8 | TRUE      | `true`  |
| 9 | RBRACE    | `}`     |
|10 | EOF       |         |

Whitespace is silently consumed (it is declared as a skip pattern in `json.tokens`).

## Token types

| Token type | Example match |
|------------|---------------|
| STRING     | `"hello"`, `"a\nb"`, `"\u0041"` |
| NUMBER     | `42`, `-1`, `3.14`, `2.5E-3` |
| TRUE       | `true` |
| FALSE      | `false` |
| NULL       | `null` |
| LBRACE     | `{` |
| RBRACE     | `}` |
| LBRACKET   | `[` |
| RBRACKET   | `]` |
| COLON      | `:` |
| COMMA      | `,` |
| EOF        | (end of input) |

## Usage

```lua
local json_lexer = require("coding_adventures.json_lexer")

local tokens = json_lexer.tokenize('{"x": 1}')
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
json.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
json_lexer  ← you are here
    ↓  feeds
json_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `json.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
