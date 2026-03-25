# JavaScript Parser

Parses JavaScript source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `excel.grammar` and delegates all parsing to the generic engine.

## How It Fits in the Stack

```
JavaScript source code
    |
    v
excel_lexer.tokenize_excel() -- tokenizes using excel.tokens
    |
    v
excel.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
excel_parser.parse_excel()   -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Usage

```python
from excel_parser import parse_excel

ast = parse_excel('let x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-excel-lexer` -- tokenizes JavaScript source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
