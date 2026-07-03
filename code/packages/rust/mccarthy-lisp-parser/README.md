# mccarthy-lisp-parser

Parser for **McCarthy's 1960 Lisp** (Lisp 1.0).

L1 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## Grammar-driven — the parser comes "for free"

This crate does **not** hand-write a recursive-descent parser.  The
S-expression grammar lives in
[`code/grammars/mccarthy_lisp.grammar`](../../../grammars/mccarthy_lisp.grammar),
which `build.rs` compiles to Rust at build time.  Parsing is two
stages — the `twig-parser` pattern:

1. **Grammar parse** — the shared [`GrammarParser`](../parser) turns
   the token stream into a generic concrete syntax tree
   (`GrammarASTNode`).  All structural rules — balanced parens,
   at-most-one dotted tail, "a dot must follow an element" — are
   enforced *by the grammar*, so there is no hand-written validation to
   drift out of sync.
2. **AST extraction** — a small extractor lowers that CST into the
   typed `LispExpr` tree below, applying the sugar expansions.

## AST

```rust
enum LispExpr {
    Nil,                                 // ()
    Symbol(String),                      // CAR
    Int(i64),                            // 42
    Cons(Box<LispExpr>, Box<LispExpr>),  // (a . b)
}
```

Two sugar expansions happen during extraction:

* `'X` → `(QUOTE X)`
* `(A B C)` → `(A . (B . (C . NIL)))` — the standard
  list-as-nested-pairs encoding from McCarthy 1960 §2.

A dotted-pair literal `(A . B)` extracts directly to
`Cons(Symbol("A"), Symbol("B"))` — no NIL terminator.

### NIL vs ()

The *symbol* `NIL` and the empty list `()` are kept distinct at this
layer: `()` → `LispExpr::Nil`, while a literal `NIL` token →
`Symbol("NIL")`.  Unifying them (as real Lisp does) is a semantic
decision deferred to the L2 `mccarthy-lisp-iir-compiler`.

## API

* `parse(src) -> Result<Vec<LispExpr>, ParseError>` — tokenize →
  grammar-parse → extract the typed AST.  A program is zero-or-more
  top-level S-expressions.
* `parse_to_cst(src) -> Result<GrammarASTNode, ParseError>` — stop at
  the generic CST (for tooling / introspection).
* `extract_program(&GrammarASTNode) -> Result<Vec<LispExpr>, ParseError>`
  — the CST → AST lowering on its own.
* `create_mccarthy_parser_from_tokens(tokens) -> GrammarParser`,
  `mccarthy_grammar() -> &'static ParserGrammar`.

## Robustness

`parse` rejects sources whose `(` nesting exceeds `MAX_PAREN_DEPTH`
(64) *before* invoking the recursive grammar parser, and the extractor
caps its own recursion at `MAX_AST_DEPTH` — both guard against
stack-overflow on pathological input.  Integer literals that overflow
`i64` are a `ParseError`, not a panic.
