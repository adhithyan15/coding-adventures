# coding-adventures-haskell-lexer (Lua)

A Haskell lexer that tokenizes Haskell source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `haskell/haskell<version>.tokens` grammar file.

## What it does

Given the input `int x = 42;`, the lexer produces:

| # | Type      | Value |
|---|-----------|-------|
| 1 | INT       | `int` |
| 2 | NAME      | `x`   |
| 3 | EQUALS    | `=`   |
| 4 | NUMBER    | `42`  |
| 5 | SEMICOLON | `;`   |
| 6 | EOF       |       |

Whitespace is silently consumed (declared as a skip pattern in the grammar).

## Version support

| Version | Haskell Release |
|---------|-------------|
| `"1.0"` | Haskell 1.0 (1996) |
| `"1.1"` | Haskell 1.1 (1997) |
| `"1.4"` | Haskell 1.4 (2002) |
| `"5"`   | Haskell 5 (2004) |
| `"7"`   | Haskell 7 (2011) |
| `"8"`   | Haskell 8 (2014) |
| `"10"`  | Haskell 10 (2018) |
| `"14"`  | Haskell 14 (2020) |
| `"17"`  | Haskell 17 (2021) |
| `"21"`  | Haskell 21 (2023) |

Default version: `"21"` (when no version is specified).

## Usage

```lua
local haskell_lexer = require("coding_adventures.haskell_lexer")

local tokens = haskell_lexer.tokenize("int x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
haskell/haskell<version>.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
haskell_lexer  ← you are here
    ↓  feeds
haskell_parser
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `haskell<version>.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
