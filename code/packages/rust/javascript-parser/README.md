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

`parse_javascript_program_division_only(source, EsVersion)` returns a typed AST
using an explicit regex-free token goal for restricted numeric frontends. It
retains the existing ASI and parser-depth protection and checks flat operator,
member and call spines before constructing recursively dropped typed nodes.
The conservative construction measure is limited to 512; ordinary list breadth
does not consume spine depth. Callers still enforce
source and AST budgets and reject unsupported constructs. General parser APIs
retain their regex rules; full context-sensitive lexical goals remain open.
This restricted numeric entry also rejects exponentiation operator tokens before
ASI parsing, preventing recursive exponent-chain stack overflow. Quoted spelling
and comment trivia do not trigger that operator check.

`parse_javascript_cst_division_only(source, EsVersion)` runs exactly the same
steps — the regex-free token goal, the exponentiation-token rejection, the ASI
parse with its depth guard, and the 512 spine bound, with the same error
texts — and stops there, returning the post-ASI `GrammarASTNode` instead of a
typed AST. It is the input of `javascript-iir-compiler` (LANG84 decision 6:
the IIR compiler walks the parse tree). `parse_javascript_program_division_only`
is now that function followed by the typed bridge, so the two always agree on
which sources parse.

The JavaScript grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, function declarations
- **expression** — arithmetic, comparison, logical, assignment, function calls, member access
- **function_declaration** — named functions with parameters and body
- **if_statement** / **while_statement** / **for_statement** — control flow
- **var_declaration** — variable declarations with initializers

## Checked provenance errors (CV02)

CV02: propagate program-root allocation and construction-event failures before accepting a parsed result; test node/event exhaustion. The caller must propagate the error and reject the compilation result.
