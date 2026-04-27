# macsyma-parser

A thin wrapper around the repo's grammar-driven `GrammarParser` that
parses MACSYMA/Maxima source into a generic `ASTNode` tree.

## What this package is

This package does the bare minimum: load `macsyma.grammar`, tokenize
the source via `macsyma-lexer`, hand both to the generic `GrammarParser`.
No hand-written recursion, no precedence-climbing code — the entire
expression grammar lives in `code/grammars/macsyma/macsyma.grammar`.

## Usage

```python
from macsyma_parser import parse_macsyma
from lang_parser import find_nodes

ast = parse_macsyma("f(x) := x^2; diff(f(x), x);")
print(ast.rule_name)  # "program"
print(len(find_nodes(ast, "statement")))  # 2
```

## The AST

The parser produces a generic `ASTNode` tree. Each node has:

- `rule_name` — the grammar rule that matched (e.g. `"program"`,
  `"statement"`, `"assign"`, `"power"`, `"postfix"`, `"atom"`).
- `children` — a list of child `ASTNode`s and `Token`s.

The grammar's precedence cascade makes the tree deeper than strictly
necessary (every expression traverses `assign → logical_or →
logical_and → logical_not → comparison → additive → multiplicative →
unary → power → postfix → atom`). The `macsyma-compiler` package
flattens this into the uniform `IRApply(head, args)` shape on its way
to the symbolic IR.

## Dependencies

- `coding-adventures-macsyma-lexer` — tokenizes the source.
- `coding-adventures-parser` — the generic grammar-driven parser.
- `coding-adventures-grammar-tools` — parses `.grammar` files.
