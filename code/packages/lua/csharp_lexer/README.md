# coding-adventures-csharp-lexer (Lua)

A C# lexer that tokenizes C# source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `csharp/csharp<version>.tokens` grammar file.

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

| Version  | C# Release          |
|----------|---------------------|
| `"1.0"`  | C# 1.0 (2002)       |
| `"2.0"`  | C# 2.0 (2005)       |
| `"3.0"`  | C# 3.0 (2007)       |
| `"4.0"`  | C# 4.0 (2010)       |
| `"5.0"`  | C# 5.0 (2012)       |
| `"6.0"`  | C# 6.0 (2015)       |
| `"7.0"`  | C# 7.0 (2017)       |
| `"8.0"`  | C# 8.0 (2019)       |
| `"9.0"`  | C# 9.0 (2020)       |
| `"10.0"` | C# 10.0 (2021)      |
| `"11.0"` | C# 11.0 (2022)      |
| `"12.0"` | C# 12.0 (2023)      |

Default version: `"12.0"` (when no version is specified).

## Usage

```lua
local csharp_lexer = require("coding_adventures.csharp_lexer")

local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

Tokenizing a specific C# version:

```lua
local tokens = csharp_lexer.tokenize_csharp("async Task Foo() {}", "5.0")
```

Getting the raw lexer object (useful for streaming or inspection):

```lua
local gl = csharp_lexer.create_csharp_lexer("int x = 1;", "12.0")
local raw_tokens = gl:tokenize()
```

## How it fits in the stack

```
csharp/csharp<version>.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
csharp_lexer  ← you are here
    ↓  feeds
csharp_parser
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `csharp<version>.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
