# coding-adventures-haskell-parser

A grammar-driven Haskell parser for the coding-adventures monorepo. It takes Haskell source text, tokenizes it with `haskell_lexer`, loads the `haskell<version>.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `int x = 5;`, the parser produces:

```
program
└── statement
    └── var_declaration
        ├── NAME       "int"
        ├── NAME       "x"
        ├── EQUALS     "="
        ├── expression
        │   └── term
        │       └── factor
        │           └── NUMBER  "5"
        └── SEMICOLON  ";"
```

The root node always has `rule_name == "program"`.

## Supported Haskell constructs

- Variable declarations: `int x = 5;`  `String y = "hello";`
- Assignments: `x = 10;`
- Expression statements: `42;`  `x;`
- Arithmetic with correct precedence: `+` and `-` at expression level, `*` and `/` at term level
- Parenthesized groups: `(a + b) * c`
- Multiple statements in a single `parse()` call

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

Default version: `"21"`.

## How it fits in the stack

```
haskell_parser  ← this package
        ↓
parser (GrammarParser)
        ↓
grammar_tools (parse_parser_grammar)
        ↓
haskell_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local haskell_parser = require("coding_adventures.haskell_parser")

-- Parse and get the AST root
local ast = haskell_parser.parse("int x = 5;")
print(ast.rule_name)  -- "program"

-- Version-specific
local ast = haskell_parser.parse("int x = 5;", "8")
```

## API

### `haskell_parser.parse(source, version) → ASTNode`

Parse a Haskell string and return the root ASTNode. Raises an error on invalid input.

### `haskell_parser.create_parser(source, version) → GrammarParser`

Tokenize and return an initialized `GrammarParser` without parsing.

### `haskell_parser.get_grammar(version) → ParserGrammar`

Return the cached `ParserGrammar` loaded from the grammar file.

## Version

0.1.0
