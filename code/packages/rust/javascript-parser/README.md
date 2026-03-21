# coding-adventures-javascript-parser

A JavaScript parser for the coding-adventures project. This crate parses JavaScript source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `javascript.grammar` file and feeds it, along with tokens from the `javascript-lexer` crate, to the generic `GrammarParser`. The grammar file defines JavaScript's syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
javascript.tokens    (grammar file)
       |
       v
javascript-lexer     (tokenizes JavaScript source → Vec<Token>)
       |
       v
javascript.grammar   (grammar file)
       |
       v
parser               (GrammarParser: builds AST from tokens + grammar)
       |
       v
javascript-parser    (THIS CRATE: wires everything together for JavaScript)
```

## Usage

```rust
use coding_adventures_javascript_parser::{create_javascript_parser, parse_javascript};

// Quick parsing — returns a GrammarASTNode
let ast = parse_javascript("var x = 1 + 2;");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_javascript_parser("function add(a, b) { return a + b; }");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The JavaScript grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, function declarations
- **expression** — arithmetic, comparison, logical, assignment, function calls, member access
- **function_declaration** — named functions with parameters and body
- **if_statement** / **while_statement** / **for_statement** — control flow
- **var_declaration** — variable declarations with initializers
