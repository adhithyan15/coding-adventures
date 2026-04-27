# ECMAScript 3 (1999) Parser

Parses ECMAScript 3 (1999) JavaScript source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `es3.grammar` file from `code/grammars/ecmascript/` and produces
`ASTNode` trees from tokenized source code.

## Usage

```python
from ecmascript_es3_parser import parse_es3

ast = parse_es3('var x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-ecmascript-es3-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
