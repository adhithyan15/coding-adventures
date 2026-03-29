# coding-adventures-typescript-parser

Grammar-driven TypeScript parser for the coding-adventures monorepo.

## What it does

This package parses TypeScript source code into an Abstract Syntax Tree (AST)
using the grammar-driven `GrammarParser` engine. It sits above the
`typescript_lexer`, `parser`, and `grammar_tools` packages in the stack.

TypeScript is a strict superset of JavaScript that adds static type annotations,
interfaces, generics, enums, and access modifiers. This parser covers the
core expression and statement subset — the same grammar rules as the JavaScript
parser, but with TypeScript-specific keyword recognition from the lexer.

## Usage

```lua
local typescript_parser = require("coding_adventures.typescript_parser")

-- Parse a TypeScript source string
local ast = typescript_parser.parse("let x = 5;")
print(ast.rule_name)  -- "program"

-- Create a parser without parsing (for trace/inspection)
local p = typescript_parser.create_parser("const PI = 3;")
local ast, err = p:parse()

-- Inspect the grammar
local g = typescript_parser.get_grammar()
print(g.rules[1].name)  -- "program"
```

## Grammar

```
program        = { statement } ;
statement      = var_declaration | assignment | expression_stmt ;
var_declaration = KEYWORD NAME EQUALS expression SEMICOLON ;
assignment     = NAME EQUALS expression SEMICOLON ;
expression_stmt = expression SEMICOLON ;
expression     = term { ( PLUS | MINUS ) term } ;
term           = factor { ( STAR | SLASH ) factor } ;
factor         = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
```

TypeScript keywords (`interface`, `type`, `enum`, `abstract`, `readonly`, etc.)
are recognized by the lexer and emitted as `KEYWORD` tokens, so they flow
naturally through the grammar without requiring new parser rules.

## Stack position

```
typescript_parser    ← this package
     ↓
   parser            — GrammarParser engine
     ↓
grammar_tools        — parse .grammar file into ParserGrammar
     ↓
typescript_lexer     — tokenize TypeScript source
     ↓
     lexer           — grammar-driven lexer engine
```

## Building and testing

```bash
cd code/packages/lua/typescript_parser
# Install dependencies, then run tests:
cat BUILD | bash
```

## Version

0.1.0
