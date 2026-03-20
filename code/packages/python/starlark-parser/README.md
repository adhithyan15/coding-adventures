# Starlark Parser

Parses Starlark source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `lang_parser` package. It demonstrates a core principle of the grammar-driven architecture: the same parser engine that parses Python can parse Starlark by simply loading a different `.grammar` file.

No new parser code is needed. The `starlark.grammar` file in `code/grammars/` declares Starlark's grammar rules in EBNF notation, and the `GrammarParser` interprets those rules at runtime.

## What Is Starlark?

Starlark is a deterministic subset of Python designed by Google for BUILD files (Bazel, Buck). It supports `def`, `for`, `if/elif/else`, list/dict comprehensions, and lambda expressions, but intentionally removes `while`, `class`, `try/except`, `import`, and recursion to guarantee termination.

## How It Fits in the Stack

```
Starlark source code
    |
    v
starlark_lexer.tokenize_starlark()     -- tokenizes using starlark.tokens
    |                                       (with indentation mode)
    v
starlark.grammar (grammar file)
    |
    v
grammar_tools.parse_parser_grammar()   -- parses the .grammar file
    |
    v
lang_parser.GrammarParser              -- generic parsing engine
    |
    v
starlark_parser.parse_starlark()       -- thin wrapper (this package)
    |
    v
ASTNode tree                           -- generic AST
```

## Usage

```python
from starlark_parser import parse_starlark

# Parse a simple assignment
ast = parse_starlark('x = 1 + 2\n')
print(ast.rule_name)  # "file"

# Parse a function definition
ast = parse_starlark('def add(x, y):\n    return x + y\n')

# Parse a BUILD-file style function call
ast = parse_starlark('cc_library(\n    name = "foo",\n    srcs = ["foo.cc"],\n)\n')

# Parse multiple statements
ast = parse_starlark('x = 1\ny = 2\n')
```

## Installation

```bash
pip install coding-adventures-starlark-parser
```

## Dependencies

- `coding-adventures-starlark-lexer` -- tokenizes Starlark source code (with indentation mode)
- `coding-adventures-parser` -- provides `GrammarParser` and `ASTNode`
- `coding-adventures-grammar-tools` -- parses `.grammar` files
