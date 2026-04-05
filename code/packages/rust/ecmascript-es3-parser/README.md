# coding-adventures-ecmascript-es3-parser

An ECMAScript 3 (1999) parser for the coding-adventures project. This crate parses ES3 JavaScript source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `es3.grammar` file and feeds it, along with tokens from the `ecmascript-es3-lexer` crate, to the generic `GrammarParser`.

## What ES3 grammar adds over ES1

- `try`/`catch`/`finally`/`throw` statements (structured error handling)
- `===` and `!==` in equality expressions (strict equality)
- `instanceof` in relational expressions
- `REGEX` as a primary expression

## How it fits in the stack

```
es3.tokens           (grammar file)
       |
       v
ecmascript-es3-lexer (tokenizes ES3 source -> Vec<Token>)
       |
       v
es3.grammar          (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
ecmascript-es3-parser (THIS CRATE: wires everything together for ES3)
```

## Usage

```rust
use coding_adventures_ecmascript_es3_parser::{create_es3_parser, parse_es3};

// Quick parsing — returns a GrammarASTNode
let ast = parse_es3("try { x(); } catch (e) { }");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_es3_parser("var result = a === b;");
let ast = parser.parse().expect("parse failed");
```
