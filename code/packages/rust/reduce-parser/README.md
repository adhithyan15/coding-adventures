# coding-adventures-reduce-parser

Reduce parser backed by `code/grammars/reduce/reduce.grammar`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the R-1-scoped subset fixed by
[MA08 §3](../../../specs/MA08-reduce-language.md). Every Reduce expression
parses down to ordinary infix/postfix operators over `head(args)`-shaped
calls, with a small statement layer (`:=`, `if`/`then`/`else`, `<< ... >>`)
sitting on top — Reduce's algebraic mode is a *statement* language, not just
an expression-and-worksheet one, unlike Derive or Wolfram.

## `:=` is genuinely one grammar rule doing two jobs

Mirroring `derive-parser`'s own identical convention: `x := 5` (variable
assignment) and `h(l, m) := x - 2*y` (procedure definition) parse through
the *same* `assignment` rule — its left-hand side is just the next-tighter
precedence level (`logical_or`), which naturally reduces down to a bare
`NAME` or a `postfix` call `h(l, m)` depending on what's actually written.
This parser does not (and does not need to) distinguish the two cases; a
future `reduce-runtime` (R-4) tells them apart by inspecting the parsed
left-hand side's shape.

## `if` and `<< ... >>` are usable as expressions

MA08 §3 confirms both are value-producing: `if b then s1 else s2` "returns
whichever branch ran", and `<< s1; s2; ... >>` "evaluates to its last
statement's value". So `expr = if_expr | group_expr | assignment` sits
*above* `assignment` in the precedence cascade — an ordered-choice (PEG)
shape mirroring `macsyma-parser`'s own `expression = if_expr | for_expr |
... | assign` — meaning `if`/`<<...>>` can appear nested inside an
assignment's right-hand side, an argument list, or each other, not just as
a standalone top-level statement.

## Precedence, loosest to tightest

```
expr = if_expr | group_expr | assignment
  assignment (:=, right-assoc, RHS is `expr`)
    → OR
      → AND
        → NOT (prefix)
          → comparison (= neq < > <= >=, flat non-chaining)
            → cons (., right-assoc -- see below)
              → additive (+ -)
                → multiplicative (* /)
                  → unary minus (prefix)
                    → power (^ / **, same operator, right-assoc)
                      → postfix application  f(...)
                        → atom  (NUMBER, NAME, {list literal}, ( ... ))
```

`=` is Reduce's *equation* operator (Boolean-valued) — never assignment;
`:=` alone owns that role. `neq`/`and`/`or`/`not` are lowercase `KEYWORD`
tokens (the mirror image of `derive-lexer`'s uppercase `AND`/`OR`/`NOT`),
matched by literal spelling.

### Where `.` (cons) binds — a gap MA08 itself leaves open

The manual's own precedence table never places `.` in the same chain this
grammar otherwise transcribes verbatim (MA08 §3 flags this explicitly).
`reduce.grammar` binds it looser than `additive` but tighter than
`comparison`: `1+2 . {3,4}` parses as `(1+2) . {3,4}`, never `1 + (2 .
{3,4})` (adding a number to a list is nonsensical), while `a . {b} = c .
{d}` still parses as an equation between two cons expressions. See
`reduce.grammar`'s own header comment for the full reasoning.

## List literals use curly braces

`{a, b, c}` (not Derive's `[a, b, c]` square brackets) is a flat list —
Reduce has no matrix-literal row-separator syntax the way Derive's vector
does, so one `arglist` production covers both a function call's arguments
and a list literal's elements. `list(a, b, c)` (the function-call spelling)
and the list accessors (`first`/`rest`/`part`/`append`/`reverse`) need no
dedicated grammar rule — they fall out of the ordinary `postfix` call
production, since they're just NAMEs followed by `(...)`.

## Usage

```rust
use coding_adventures_reduce_parser::parse_reduce;

let ast = parse_reduce("h(l, m) := l + m;\nh(1, 2);\n");
assert_eq!(ast.rule_name, "program");
```

`parse_reduce` panics on a malformed source string; use `try_parse_reduce`
for the `Result`-returning form, or `create_reduce_parser` directly if you
need the raw `GrammarParser`.

## Recursion-depth guard: five shapes, and a rule-frame surprise

`reduce.grammar` has five distinct self-referential (right-recursive)
productions — parenthesised grouping, a `:=` chain, an `if`/`else` chain, a
cons (`.`) chain, and a power (`^`) chain — each measured independently
(binary search, uncapped parser, default-stack worker thread, debug build),
per [MA06](../../../specs/MA06-j-language.md) §6's established methodology.
Every "flat chain of one operator" production that uses EBNF `{ x }`
repetition instead (`logical_or`, `logical_and`, `additive`,
`multiplicative`, `postfix`'s call chain, `arglist`, `group_expr`'s
statement sequence) was directly confirmed *not* to cost native stack
regardless of width — a throwaway probe grammar built from
`GrammarElement::Repetition` alone parsed one million repeated items on a
default-stack thread with zero crashes, since `match_element`'s own
`Repetition` arm is a plain iterative loop.

The genuine surprise: converting each shape's nesting-count crash floor
into rule-frame terms (the units `MAX_RULE_DEPTH` actually bounds) shows
the cons chain — which tolerates the *most* nesting levels of the five
(163) — has the *lowest* rule-frame floor (179), because each `cons` link's
persistent per-level cost is cheaper (one rule-frame) than the other four
shapes' (two), yet its own call chain evidently costs more native-stack
bytes per crossing. Parenthesised nesting — the shape that binds for nearly
every sibling `*-parser` crate in this repo, and would naively look like
Reduce's own binding shape too (crashing at the fewest *levels*, 19) — is
*not* the binding constraint here once measured in the frame terms that
actually matter. See `MAX_RULE_DEPTH`'s own doc comment in `src/lib.rs` for
the full measurement table and reasoning.

## Where this fits

`reduce-parser` is R-3 of Reduce's frontend crates, consuming the token
stream from `reduce-lexer` (R-2) against
`code/grammars/reduce/reduce.grammar` to build the `GrammarASTNode` CST a
future `reduce-runtime` (R-4) will lower into `symbolic_ir::IRNode` and
evaluate with `symbolic_vm::VM`.
