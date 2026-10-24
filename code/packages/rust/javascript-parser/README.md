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

`parse_javascript_cst_bounded(source, EsVersion)` returns the post-ASI
`GrammarASTNode` with the bounds `javascript-iir-compiler` relies on (LANG84
decision 6: the IIR compiler walks the parse tree). It lexes with the ordinary
moded lexer, so the edition's F10 table decides regex versus division; it
rejects `**`/`**=` tokens before parsing (exponentiation's rule is
right-recursive, so a hostile chain could exhaust the stack first; quoted
spelling and comments do not trigger the check); it runs the ASI parse with
its depth guard; and it bounds flat operator, member and call spines to 512
(ordinary list breadth does not count). Callers still enforce source budgets
and reject unsupported constructs. Since 0.65.0 it replaces
`parse_javascript_cst_division_only` and
`parse_javascript_program_division_only`, which lexed with `REGEX` deleted
from the compiled grammar (LANG84 phase 6a).

The JavaScript grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, function declarations
- **expression** — arithmetic, comparison, logical, assignment, function calls, member access
- **function_declaration** — named functions with parameters and body
- **if_statement** / **while_statement** / **for_statement** — control flow
- **var_declaration** — variable declarations with initializers

## Checked provenance errors (CV02)

CV02: propagate program-root allocation and construction-event failures before accepting a parsed result; test node/event exhaustion. The caller must propagate the error and reject the compilation result.
