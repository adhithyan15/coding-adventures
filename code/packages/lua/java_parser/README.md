# coding-adventures-java-parser

A grammar-driven Java parser for the coding-adventures monorepo. It takes Java source text, tokenizes it with `java_lexer`, loads the `java<version>.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

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

## Supported Java constructs

- Variable declarations: `int x = 5;`  `String y = "hello";`
- Assignments: `x = 10;`
- Expression statements: `42;`  `x;`
- Arithmetic with correct precedence: `+` and `-` at expression level, `*` and `/` at term level
- Parenthesized groups: `(a + b) * c`
- Multiple statements in a single `parse()` call

## Version support

| Version | Java Release |
|---------|-------------|
| `"1.0"` | Java 1.0 (1996) |
| `"1.1"` | Java 1.1 (1997) |
| `"1.4"` | Java 1.4 (2002) |
| `"5"`   | Java 5 (2004) |
| `"7"`   | Java 7 (2011) |
| `"8"`   | Java 8 (2014) |
| `"10"`  | Java 10 (2018) |
| `"14"`  | Java 14 (2020) |
| `"17"`  | Java 17 (2021) |
| `"21"`  | Java 21 (2023) |

Default version: `"21"`.

## How it fits in the stack

```
java_parser  ← this package
        ↓
parser (GrammarParser)
        ↓
grammar_tools (parse_parser_grammar)
        ↓
java_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local java_parser = require("coding_adventures.java_parser")

-- Parse and get the AST root
local ast = java_parser.parse("int x = 5;")
print(ast.rule_name)  -- "program"

-- Version-specific
local ast = java_parser.parse("int x = 5;", "8")
```

## API

### `java_parser.parse(source, version) → ASTNode`

Parse a Java string and return the root ASTNode. Raises an error on invalid input.

### `java_parser.create_parser(source, version) → GrammarParser`

Tokenize and return an initialized `GrammarParser` without parsing.

### `java_parser.get_grammar(version) → ParserGrammar`

Return the cached `ParserGrammar` loaded from the grammar file.

## Version

0.1.0
