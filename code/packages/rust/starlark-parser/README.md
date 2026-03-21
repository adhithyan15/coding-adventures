# coding-adventures-starlark-parser

A Starlark parser for the coding-adventures project. This crate parses Starlark source code into an abstract syntax tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

The parsing pipeline has four stages:

```text
Source code ("x = 1 + 2")
       |
       v
starlark-lexer       (tokenizes source into Token stream)
       |
       v
starlark.grammar     (grammar file defining Starlark syntax)
       |
       v
grammar-tools        (parses .grammar into ParserGrammar struct)
       |
       v
parser::GrammarParser  (parses tokens into GrammarASTNode tree)
```

This crate is the glue that wires all four stages together for Starlark.

## How it fits in the stack

```
starlark-lexer   (tokenization)
       |
       v
starlark-parser  (THIS CRATE: parsing)
       |
       v
[future]         (type checking, evaluation, code generation)
```

## Usage

```rust
use coding_adventures_starlark_parser::{create_starlark_parser, parse_starlark};

// Quick parsing — returns a GrammarASTNode
let ast = parse_starlark("x = 1 + 2\n");
println!("{:?}", ast.rule_name);  // "file"

// Or get the parser object for more control
let mut parser = create_starlark_parser("def f():\n    return 1\n");
let ast = parser.parse().expect("parse failed");
```

## What the AST looks like

The grammar-driven parser produces a generic `GrammarASTNode` tree where:
- Each node has a `rule_name` (e.g., "file", "statement", "expression")
- Children are either nested nodes or raw tokens
- The tree structure follows the grammar rules in `starlark.grammar`

## Supported Starlark constructs

- **Simple statements**: assignment, return, break, continue, pass, load
- **Compound statements**: if/elif/else, for, def
- **Expressions**: arithmetic, comparison, boolean logic, bitwise ops
- **Literals**: integers, floats, strings, lists, dicts, tuples
- **Functions**: definitions with default/varargs/kwargs parameters, calls
- **Comprehensions**: list and dict comprehensions
- **Augmented assignment**: `+=`, `-=`, `*=`, etc.
