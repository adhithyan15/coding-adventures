# ECMAScript 1 (1997) Parser

Parses ECMAScript 1 (1997) JavaScript source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `es1.grammar` file from `code/grammars/ecmascript/` and produces
`ASTNode` trees from tokenized source code.

## Usage

```python
from ecmascript_es1_parser import parse_es1

ast = parse_es1('var x = 1 + 2;')
print(ast.rule_name)  # "program"
```

## Dependencies

- `coding-adventures-ecmascript-es1-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
