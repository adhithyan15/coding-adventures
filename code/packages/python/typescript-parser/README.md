# TypeScript Parser

Parses TypeScript source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `typescript.grammar` and delegates all parsing to the generic engine.

## How It Fits in the Stack

```
TypeScript source code
    |
    v
typescript_lexer.tokenize_typescript() -- tokenizes using typescript.tokens
    |
    v
typescript.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
typescript_parser.parse_typescript()   -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Usage

```python
from typescript_parser import parse_typescript

ast = parse_typescript('let x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-typescript-lexer` -- tokenizes TypeScript source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
