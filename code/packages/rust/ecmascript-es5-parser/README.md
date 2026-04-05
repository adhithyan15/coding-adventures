# coding-adventures-ecmascript-es5-parser

An ECMAScript 5 (2009) parser for the coding-adventures project. This crate parses ES5 JavaScript source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `es5.grammar` file and feeds it, along with tokens from the `ecmascript-es5-lexer` crate, to the generic `GrammarParser`.

## What ES5 grammar adds over ES3

- `debugger` statement (`debugger;` acts as a breakpoint)
- Getter/setter properties in object literals (`{ get name() {}, set name(v) {} }`)

## How it fits in the stack

```
es5.tokens           (grammar file)
       |
       v
ecmascript-es5-lexer (tokenizes ES5 source -> Vec<Token>)
       |
       v
es5.grammar          (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
ecmascript-es5-parser (THIS CRATE: wires everything together for ES5)
```

## Usage

```rust
use coding_adventures_ecmascript_es5_parser::{create_es5_parser, parse_es5};

// Quick parsing — returns a GrammarASTNode
let ast = parse_es5("debugger;");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_es5_parser("try { x(); } catch (e) { debugger; }");
let ast = parser.parse().expect("parse failed");
```
