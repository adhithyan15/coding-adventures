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

The JavaScript grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, function declarations
- **expression** — arithmetic, comparison, logical, assignment, function calls, member access
- **function_declaration** — named functions with parameters and body
- **if_statement** / **while_statement** / **for_statement** — control flow
- **var_declaration** — variable declarations with initializers

## Checked provenance errors (CV02)

CV02: propagate program-root allocation and construction-event failures before accepting a parsed result; test node/event exhaustion. The caller must propagate the error and reject the compilation result.

## Optional array occurrence observation (CLOC32)

`bridge::grammar_to_program_with_array_observer` borrows a caller-owned
`ArrayOccurrenceObserver: Send` and the original grammar CST. Its callback
receives each actual array occurrence once after its children; its optional
returned identity is assigned only to `ArrayExpression.cv`. Ordinary
`grammar_to_program` retains existing leaf IDs and leaves composites untraced.

The observed entry distinguishes bridge, sink and worker failures. Its scoped
128 MiB worker rejects partial candidates before returning an error to a small
caller; the first sink refusal remains authoritative. Arbitrary Rust callback
work, returned IDs and successful AST ownership remain caller responsibilities.
This hook introduces no source/span namespace and does not certify arbitrary
sink metadata or wire the CLI. A trusted source-range adapter is follow-up work.
Final-source Miri remains DEFERRED/PENDING; existing frozen validation is untouched.
