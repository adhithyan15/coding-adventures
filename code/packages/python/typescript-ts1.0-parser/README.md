# TypeScript 1.0 (April 2014) Parser

Parses TypeScript 1.0 source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `ts1.0.grammar` file from `code/grammars/typescript/` and produces
`ASTNode` trees from tokenized source code.

## Usage

```python
from typescript_ts10_parser import parse_ts10

ast = parse_ts10('interface Foo { name: string; }')
print(ast.rule_name)  # "program"
```

Using the factory for advanced usage:

```python
from typescript_ts10_parser import create_ts10_parser

parser = create_ts10_parser('var x: number = 1;')
ast = parser.parse()
```

## AST Node Examples

```python
from typescript_ts10_parser import parse_ts10

def find_nodes(node, rule_name):
    results = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        results.extend(find_nodes(child, rule_name))
    return results

ast = parse_ts10('interface Point { x: number; y: number; }')
interfaces = find_nodes(ast, 'interface_declaration')
# [ASTNode('interface_declaration', ...)]
```

## Grammar Rules (TypeScript-specific)

| Rule | Example |
|------|---------|
| `interface_declaration` | `interface Foo { x: string; }` |
| `type_alias_declaration` | `type Alias = string;` |
| `enum_declaration` | `enum Color { Red, Green, Blue }` |
| `namespace_declaration` | `namespace MyNS { }` |
| `ambient_declaration` | `declare var x: number;` |
| `ts_class_declaration` | `class Animal { name: string; }` |

## Dependencies

- `coding-adventures-typescript-ts1.0-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
