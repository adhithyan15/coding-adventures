# coding-adventures-ruby-parser

A Ruby parser for the coding-adventures project. This crate parses Ruby source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `ruby.grammar` file and feeds it, along with tokens from the `ruby-lexer` crate, to the generic `GrammarParser`. The grammar file defines Ruby's syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
ruby.tokens      (grammar file)
       |
       v
ruby-lexer       (tokenizes Ruby source → Vec<Token>)
       |
       v
ruby.grammar     (grammar file)
       |
       v
parser           (GrammarParser: builds AST from tokens + grammar)
       |
       v
ruby-parser      (THIS CRATE: wires everything together for Ruby)
```

## Usage

```rust
use coding_adventures_ruby_parser::{create_ruby_parser, parse_ruby};

// Quick parsing — returns a GrammarASTNode
let ast = parse_ruby("x = 1 + 2");
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_ruby_parser("def add(a, b)\n  a + b\nend");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The Ruby grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — assignments, expression statements, method definitions, class definitions, control flow
- **expression** — arithmetic, comparison, logical, method calls, member access
- **def_statement** — method definitions with parameters and body, terminated by `end`
- **class_statement** — class definitions with methods and body, terminated by `end`
- **if_statement** / **while_statement** — control flow with `end` terminators
