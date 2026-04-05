# coding-adventures-ecmascript-es1-parser

An ECMAScript 1 (1997) parser for the coding-adventures project. This crate parses ES1 JavaScript source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `es1.grammar` file and feeds it, along with tokens from the `ecmascript-es1-lexer` crate, to the generic `GrammarParser`. The grammar file defines ES1's syntactic structure in a declarative EBNF format.

## What ES1 grammar supports

- Variable declarations (`var x = 1;`)
- Function declarations and expressions
- All 14 ES1 statement types (if, while, for, for-in, switch, with, etc.)
- Full expression precedence chain with `==`/`!=` (no `===`/`!==`)
- Object and array literals

## How it fits in the stack

```
es1.tokens           (grammar file)
       |
       v
ecmascript-es1-lexer (tokenizes ES1 source -> Vec<Token>)
       |
       v
es1.grammar          (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
ecmascript-es1-parser (THIS CRATE: wires everything together for ES1)
```

## Usage

```rust
use coding_adventures_ecmascript_es1_parser::{create_es1_parser, parse_es1};

// Quick parsing — returns a GrammarASTNode
let ast = parse_es1("var x = 1 + 2;");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_es1_parser("function add(a, b) { return a + b; }");
let ast = parser.parse().expect("parse failed");
```
