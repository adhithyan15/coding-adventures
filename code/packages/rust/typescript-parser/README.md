# coding-adventures-typescript-parser

A TypeScript parser for the coding-adventures project. This crate parses TypeScript source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `typescript.grammar` file and feeds it, along with tokens from the `typescript-lexer` crate, to the generic `GrammarParser`. The grammar file defines TypeScript's syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
typescript.tokens    (grammar file)
       |
       v
typescript-lexer     (tokenizes TypeScript source → Vec<Token>)
       |
       v
typescript.grammar   (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
typescript-parser    (THIS CRATE: wires everything together for TypeScript)
```

## Usage

```rust
use coding_adventures_typescript_parser::{create_typescript_parser, parse_typescript};

// Quick parsing — returns a GrammarASTNode
let ast = parse_typescript("let x: number = 1 + 2;");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_typescript_parser("function add(a: number, b: number): number { return a + b; }");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The TypeScript grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, function declarations, interface declarations
- **expression** — arithmetic, comparison, logical, assignment, function calls, member access
- **function_declaration** — functions with typed parameters and return type annotations
- **interface_declaration** — TypeScript interface definitions with typed members
- **type_annotation** — colon followed by a type expression
- **var_declaration** — variable declarations with optional type annotations
- **if_statement** / **while_statement** / **for_statement** — control flow
