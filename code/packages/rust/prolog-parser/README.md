# prolog-parser (Rust)

Grammar-driven ISO/Core Prolog parser. Consumes `prolog-lexer` tokens
and the syntactic grammar from `code/grammars/prolog/iso.grammar`,
produces an AST, and lowers it to `logic_core` terms ready for the
`logic-engine`.

## Architecture

```text
   Prolog source text
        │
        ▼
   prolog-lexer (grammar-driven)
        │
        ▼
   parser::GrammarParser + iso.grammar
        │
        ▼
   GrammarASTNode
        │
        ▼ ast_to_term / collect_clauses_and_queries
        ▼
   logic_core::Term + ProgramItem
```

Mirrors `code/packages/python/iso-prolog-parser` exactly: same
`iso.grammar` file, same `GrammarParser` machinery. Token and AST
streams agree by construction.

## API

```rust
use prolog_parser::{parse_iso_prolog, collect_clauses_and_queries, ProgramItem};

let src = "father(homer, bart).\nparent(X, Y) :- father(X, Y).";
let ast = parse_iso_prolog(src);
let items = collect_clauses_and_queries(&ast);
for item in &items {
    match item {
        ProgramItem::Fact(term)          => { /* ... */ }
        ProgramItem::Rule { head, body } => { /* ... */ }
        ProgramItem::Query(body)         => { /* ... */ }
    }
}
```

For recoverable errors, use `try_parse_iso_prolog`:

```rust
match prolog_parser::try_parse_iso_prolog(source) {
    Ok(ast) => { /* ... */ }
    Err(e)  => { /* e.message etc. */ }
}
```

## What's Supported

Per `iso.grammar` (the canonical syntactic grammar):

- **Facts**: `father(homer, bart).`
- **Rules**: `parent(X, Y) :- father(X, Y).`
- **Multi-literal bodies**: `gp(X, Z) :- parent(X, Y), parent(Y, Z).`
- **Queries**: `?- parent(homer, Who).`
- **Lists**: `[a, b, c]`, `[H | T]`, `[a, b | Rest]`, `[]`
- **Compound terms**: `pair(left(X), right(Y))`
- **Equality goals**: `X = Y`, `X \= Y`
- **Cut**: `!`
- **Atoms, integers, floats, strings, variables, anonymous `_`**

## What's NOT in This Slice

- **Operator-precedence parsing** (`X + Y * Z`, `X = 1 + 2`). The
  Python ecosystem keeps this in a separate `prolog-operator-parser`
  crate; the Rust mirror is a planned follow-up. Until then, use
  canonical functional form for non-trivial expressions.
- **User-defined operator directives**.
- **DCG transformations** (`-->`): lower to a placeholder Fact in
  this slice; a future spec handles the DCG-to-Prolog rewrite.
- **Negation-as-failure** (`\+ G`): parses as a compound term
  `'\+'(G)`; the downstream `prolog-loader` will translate this into
  the engine's `BodyLiteral::Neg`.

## Variable Identity

Within a single clause, the same variable name shares one
`logic_core::LogicVar`. Across clauses, identity is fresh.
`collect_clauses_and_queries` handles this automatically by creating
a fresh `var_map` per statement. If you call `ast_to_term` directly,
pass a fresh `HashMap<String, LogicVar>` for each clause.

Anonymous variables (`_`) bypass the map and always get a fresh
`LogicVar` per occurrence — Prolog semantics.

## Regenerating the Embedded Grammar

```sh
cargo run -p prolog-parser --example regenerate_grammar
```

Reads `code/grammars/prolog/iso.grammar`, parses it via
`grammar-tools`, compiles to Rust source, writes `src/_grammar.rs`.
The generated file is checked into the repository.

## Status

Experimental. Covers enough of the language to round-trip facts,
rules, queries, and lists through the Rust runtime. Operator-precedence
parsing is the next major work item.
