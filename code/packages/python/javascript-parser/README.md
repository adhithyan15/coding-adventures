# JavaScript Parser

Parses JavaScript source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It loads `javascript.grammar` and delegates all parsing to the generic engine.

## How It Fits in the Stack

```
JavaScript source code
    |
    v
javascript_lexer.tokenize_javascript() -- tokenizes using javascript.tokens
    |
    v
javascript.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
javascript_parser.parse_javascript()   -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Usage

```python
from javascript_parser import parse_javascript

ast = parse_javascript('let x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-javascript-lexer` -- tokenizes JavaScript source code
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
