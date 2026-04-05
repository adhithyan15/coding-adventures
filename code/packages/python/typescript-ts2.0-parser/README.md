# TypeScript 2.0 (September 2016) Parser

Parses TypeScript 2.0 source code into Abstract Syntax Trees (ASTs).

## Overview

This package is a thin wrapper around the generic `GrammarParser`. It loads
the `ts2.0.grammar` file from `code/grammars/typescript/` and produces
`ASTNode` trees from tokenized TypeScript 2.0 source code.

## Usage

```python
from typescript_ts20_parser import parse_ts20

ast = parse_ts20('const double = (x: number): number => x * 2;')
print(ast.rule_name)  # "program"
```

Using the factory for advanced usage:

```python
from typescript_ts20_parser import create_ts20_parser

parser = create_ts20_parser('import { Foo } from "./foo";')
ast = parser.parse()
```

## AST Node Examples

```python
from typescript_ts20_parser import parse_ts20

def find_nodes(node, rule_name):
    results = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        results.extend(find_nodes(child, rule_name))
    return results

# Find all arrow functions
ast = parse_ts20('const double = (x: number) => x * 2;')
arrows = find_nodes(ast, 'arrow_function')

# Find all imports
ast = parse_ts20('import { Foo } from "./foo"; import Bar from "./bar";')
imports = find_nodes(ast, 'import_declaration')
print(len(imports))  # 2
```

## Grammar Rules

### New in TS 2.0 (ES2015 baseline)

| Rule | Example |
|------|---------|
| `arrow_function` | `(x: number) => x * 2` |
| `class_declaration` | `class Foo extends Bar {}` |
| `import_declaration` | `import { Foo } from "./foo"` |
| `export_declaration` | `export default function foo() {}` |
| `for_of_statement` | `for (const x of arr) {}` |

### Inherited from TS 1.0

| Rule | Example |
|------|---------|
| `interface_declaration` | `interface Foo { x: string; }` |
| `type_alias_declaration` | `type Alias = string \| never;` |
| `enum_declaration` | `enum Color { Red, Green, Blue }` |
| `namespace_declaration` | `namespace MyNS { }` |
| `ambient_declaration` | `declare function fail(): never;` |
| `ts_class_declaration` | `class Animal { name: string; }` |

## Dependencies

- `coding-adventures-typescript-ts2.0-lexer` — Tokenizes source code
- `coding-adventures-grammar-tools` — Parses `.grammar` files
- `coding-adventures-parser` — Provides `GrammarParser` engine
