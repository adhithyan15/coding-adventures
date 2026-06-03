# mccarthy-lisp-parser

Parser for **McCarthy's 1960 Lisp** (Lisp 1.0).

L1 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## What it does

Turns the token stream from `mccarthy-lisp-lexer` into an
S-expression AST shaped exactly like McCarthy's 1960 definition:

```rust
enum LispExpr {
    Nil,                                 // ()
    Symbol(String),                      // CAR
    Int(i64),                            // 42
    Cons(Box<LispExpr>, Box<LispExpr>),  // (a . b)
}
```

Two sugar expansions happen at parse time:

* `'X` → `(QUOTE X)`
* `(A B C)` → `(A . (B . (C . NIL)))` — the standard list-as-nested-pairs encoding from McCarthy 1960 §2.

A dotted-pair literal `(A . B)` parses directly to `Cons(Symbol("A"), Symbol("B"))` — no NIL terminator.

## API

* `parse(src) -> Result<Vec<LispExpr>, ParseError>` — top-level convenience.
* `parse_tokens(toks) -> Result<Vec<LispExpr>, ParseError>` — same, but on an already-tokenized stream.

A program is zero-or-more S-expressions (matching McCarthy's "program = sequence of forms" reading).
