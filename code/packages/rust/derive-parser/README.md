# coding-adventures-derive-parser

Derive parser backed by `code/grammars/derive/derive.grammar`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the D-1-scoped subset fixed by
[MA07 §3](../../../specs/MA07-derive-language.md). Every Derive expression
parses down to ordinary infix/postfix operators over `head(args)`-shaped
calls — there is no `f[x]`-style universal application syntax and no
pattern/rewrite-rule vocabulary the way Wolfram has one; Derive's
"transform this expression" operations (`DIF`, `INT`, `LIM`, `SOLVE`, …)
are each their own named function, called with ordinary parentheses.

## `:=` is genuinely one grammar rule doing two jobs

Mirroring `derive-lexer`'s own README: `x := 5` (variable assignment) and
`F(x) := x^2 + 1` (function definition) parse through the *identical*
`assignment` rule — its left-hand side is just the next-tighter precedence
level (`logical_or`), which naturally reduces down to a bare `NAME` or a
`postfix` application `F(x)` depending on what's actually written. This
parser does not (and does not need to) distinguish the two cases; a future
`derive-runtime` (D-4) tells them apart by inspecting the parsed left-hand
side's shape.

## Precedence, loosest to tightest

```
assignment (:=, right-assoc)
  → OR
    → AND
      → NOT (prefix)
        → comparison (= <= < > >=)
          → additive (+ -)
            → multiplicative (* /)
              → unary minus (prefix)
                → power (^, right-assoc)
                  → postfix application  F(...)
                    → atom  (NUMBER, NAME, vector, ( ... ))
```

`=` is Derive's *equation* operator (an `Equal`-headed Boolean-valued
expression, e.g. `SOLVE`'s first argument) — never assignment; `:=` alone
owns that role, at the loosest level. This mirrors Macsyma's identical
`=`-is-equation / `:`-or-`:=`-is-assignment split, minus Macsyma's separate
bare-`:` form (Derive only ever uses `:=`, for both variables and
functions).

## Vector/matrix literals

`[a, b, c]` is a one-row vector; `[a, b, c; d, e, f]` is a matrix, rows
separated by `;` (the *only* use of `;` in this subset — Derive has no
general statement terminator). Both parse through the same `vector` rule;
the row count (one vs. more than one) is what a future D-4/D-5 lowering
uses to decide between a flat `List[...]` and a `List` of row-`List`s.

## Usage

```rust
use coding_adventures_derive_parser::parse_derive;

let ast = parse_derive("F(x) := DIF(SIN(x), x)\nF(0)\n");
assert_eq!(ast.rule_name, "program");
```

`parse_derive` panics on a malformed source string; use
`try_parse_derive` for the `Result`-returning form, or
`create_derive_parser` directly if you need the raw `GrammarParser`.

## Where this fits

`derive-parser` is D-3 of Derive's frontend crates, consuming the token
stream from `derive-lexer` (D-2) against
`code/grammars/derive/derive.grammar` to build the `GrammarASTNode` CST a
future `derive-runtime` (D-4) will lower into `symbolic_ir::IRNode` and
evaluate with `symbolic_vm::VM`.
