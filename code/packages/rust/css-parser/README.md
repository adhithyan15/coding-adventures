# coding-adventures-css-parser

A CSS parser for the coding-adventures project. This crate parses CSS source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `css.grammar` file and feeds it, along with tokens from the `css-lexer` crate, to the generic `GrammarParser`. The grammar file defines CSS's syntactic structure — stylesheets, rules, selectors, declarations, at-rules, and values — in a declarative EBNF format.

## How it fits in the stack

```
css.tokens       (grammar file)
       |
       v
css-lexer        (tokenizes CSS source → Vec<Token>)
       |
       v
css.grammar      (grammar file)
       |
       v
parser           (GrammarParser: builds AST from tokens + grammar)
       |
       v
css-parser       (THIS CRATE: wires everything together for CSS)
```

## Usage

```rust
use coding_adventures_css_parser::{create_css_parser, parse_css};

// Quick parsing — returns a GrammarASTNode
let ast = parse_css("body { color: red; }");
assert_eq!(ast.rule_name, "stylesheet");

// Or get the parser object for more control
let mut parser = create_css_parser("h1 { font-size: 16px; }");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The CSS grammar covers:

- **stylesheet** — the top-level rule, a sequence of rules
- **rule** — either a qualified rule or an at-rule
- **qualified_rule** — selector list + declaration block
- **at_rule** — at-keyword + prelude + block or semicolon
- **selector_list** — comma-separated selectors (type, class, ID, attribute, pseudo)
- **declaration** — property: value pairs
- **block** — curly-brace delimited content
