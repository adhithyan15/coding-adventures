# ECMAScript 5 (2009) Parser

Parses ECMAScript 5 (2009) JavaScript source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `es5.grammar` file from `code/grammars/ecmascript/` and produces
`ASTNode` trees from tokenized source code.

## Usage

```python
from ecmascript_es5_parser import parse_es5

ast = parse_es5('var x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-ecmascript-es5-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
