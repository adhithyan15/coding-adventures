# coding-adventures-csharp-parser (Lua)

A C# parser that builds an Abstract Syntax Tree (AST) from C# source text. It is a grammar-driven wrapper that tokenizes C# with `csharp_lexer`, loads the `csharp<version>.grammar` production rules with `grammar_tools`, and produces a structured AST using the `GrammarParser` from `coding-adventures-parser`.

## What it does

Given the input `int x = 1 + 2 * 3;`, the parser produces:

```
program
└── statement
    └── var_declaration
        ├── INT      "int"
        ├── NAME     "x"
        ├── EQUALS   "="
        ├── expression
        │   ├── term
        │   │   └── factor
        │   │       └── NUMBER  "1"
        │   ├── PLUS  "+"
        │   └── term
        │       ├── factor
        │       │   └── NUMBER  "2"
        │       ├── STAR  "*"
        │       └── factor
        │           └── NUMBER  "3"
        └── SEMICOLON  ";"
```

The tree captures operator precedence: `*` binds tighter than `+` because `term` nests inside `expression`.

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
local csharp_parser = require("coding_adventures.csharp_parser")

-- Parse with the default version (12.0)
local ast = csharp_parser.parse_csharp("int x = 1 + 2;")
print(ast.rule_name)  -- "program"

-- Parse with a specific version
local ast8 = csharp_parser.parse_csharp("int x = 1;", "8.0")

-- Create a parser object without immediately parsing
local p = csharp_parser.create_csharp_parser("int x = 42;", "5.0")
local ast2, err = p:parse()

-- Inspect the grammar rules
local g = csharp_parser.get_grammar()
print(g.rules[1].name)  -- "program"
```

## Grammar subset

The grammar covers the core of C# expressions that is valid across all versions:

```
program        = { statement } ;
statement      = var_declaration | assignment | expression_stmt ;
var_declaration = NAME NAME EQUALS expression SEMICOLON ;
assignment     = NAME EQUALS expression SEMICOLON ;
expression_stmt = expression SEMICOLON ;
expression     = term { ( PLUS | MINUS ) term } ;
term           = factor { ( STAR | SLASH ) factor } ;
factor         = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
```

## How it fits in the stack

```
csharp/csharp<version>.grammar  (code/grammars/)
    ↓  parsed by grammar_tools
ParserGrammar
    ↓  drives
GrammarParser  (coding-adventures-parser)
    ↓  fed tokens by
csharp_lexer  (coding-adventures-csharp-lexer)
    ↓  assembled by
csharp_parser  ← you are here
```

## Dependencies

- `coding-adventures-csharp-lexer` — tokenizes C# source text
- `coding-adventures-parser` — provides `GrammarParser`
- `coding-adventures-grammar-tools` — parses `csharp<version>.grammar`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
