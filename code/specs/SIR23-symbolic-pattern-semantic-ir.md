# SIR23 — Symbolic expression + pattern/rewrite IR extension (CAS math languages)

## Motivation

[`SIR10`](SIR10-narrow-waist-semantic-ir.md) has no symbolic-expression
vocabulary: no "apply a head to args as data" concept, no pattern variables,
no rewrite rules. This spec adds that vocabulary, additively, following the
same discipline [`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md)
used for Python/JS. Every existing module and backend match arm stays valid;
this only adds new `Expr` variants and `Feature` flags.

This is Stream B of [`HML01`](HML01-math-to-semantic-ir.md) — the substrate
for `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, and
`maxima-to-semantic-ir`, and for every future symbolic-CAS-family frontend
(Reduce, Derive, Maple) per
[`HML00`](HML00-historical-math-languages-roadmap.md).

**The fidelity decision this spec commits to:** the IR carries **full
rewrite semantics** — patterns and rules are first-class data, and
`ReplaceAll`/`ReplaceRepeated` are IR-level operations a backend must
actually execute (via a real pattern matcher in its runtime), not a
frontend-side operation that only ever lowers a precomputed answer. This
was an explicit repo-owner choice over the cheaper "evaluate with the
existing CAS engine, then lower only the final `IRNode` result as inert
data" alternative — the cheaper path can't compile an *uncomputed* Wolfram
function body (one that pattern-matches at call time) into equivalent JS
logic; this spec's path can.

Every new node kind is mapped **1:1** onto `symbolic_ir::IRNode`'s existing
five-variant shape (`Symbol`/`Integer`/`Rational`/`Float`/`Str`/`Apply`), so
lowering a Wolfram or Macsyma parse tree into these nodes is mechanical.

## Scope

**In scope:**

- Symbolic expression application: `head[args…]` / `head(args…)` as data,
  not just as a call — the same expression can appear as a value, a pattern
  target, or a rewrite-rule left-hand side
- Pattern blanks: `_` (`Blank`), `x_` (named `Pattern`), `_h` / `x_h`
  (head-constrained blank)
- Rules: `a -> b` (`Rule`, eager RHS) and `a :> b` (`RuleDelayed`, RHS
  re-evaluated per match)
- Replacement: `expr /. rules` (`ReplaceAll`, one pass) and `expr //. rules`
  (`ReplaceRepeated`, fixed point)
- Exact rational and complex scalars (`SirType::Rational`/`SirType::Complex`
  — shared with [`SIR22`](SIR22-array-matrix-semantic-ir.md), landed once)

**Explicitly out of scope (deferred):**

- Sequence patterns (`__`/`___`, `BlankSequence`/`BlankNullSequence`),
  `Repeated`, `Except`, `Longest`/`Shortest`, `Replace` level-specs — these
  are still open even in Wolfram's *native* pipeline per
  [`MA04`](MA04-wolfram-language.md) (W-20's own deferred list); SIR23
  tracks that scope exactly rather than getting ahead of the native frontend.
- Arbitrary assumption contexts (`assume(x >= 0)`) as IR-level data — the
  frontend resolves what it can at lowering time; an unresolved assumption
  stays an unevaluated `SymApply`, exactly as the native runtimes do today.
- The `cas-*` algorithm surface itself (`Expand`/`Factor`/`Solve`/`D`/
  `Integrate`, …) is **not** new IR — those are ordinary `SymApply` nodes
  with well-known heads (`"Expand"`, `"Factor"`, …); whether a *backend*
  can evaluate them is a runtime-library question (does
  `sir-runtime-symbolic` implement a `Factor` evaluator?), not an IR
  question. First cut ships pattern-matching + rewriting infrastructure;
  wiring the full `cas-*` algorithm library into `sir-runtime-symbolic` is
  separate, later work, matching how the native Wolfram frontend itself
  still has this as an open "Future" item.

## New `SirType` variants

```text
SirType::SymExpr    -- an opaque symbolic-expression handle; carries no
                        static shape (mirrors symbolic_ir::IRNode's own
                        dynamically-shaped tree)
SirType::Rational    -- shared with SIR22
SirType::Complex     -- shared with SIR22
```

## New `Expr` variants

All new variants carry `span`. The five core variants are named to mirror
`symbolic_ir::IRNode` one-for-one:

```text
SymSymbol { name: String, span }                  -- IRNode::Symbol
SymRational { numer: i64, denom: i64, span }      -- IRNode::Rational
                                                      (reduced form; frontend
                                                      normalizes exactly as
                                                      IRNode::rational does)
SymApply {
    head: Box<Expr>,       -- usually a SymSymbol, but a computed head
                              (`f[x][y]`) is legal — head is an Expr, not
                              a bare string, unlike SIR22's simpler shapes
    args: Vec<Expr>,
    span,
}
```

(`IntLit`/`FloatLit`/`StrLit` already exist in SIR10/SIR16 and are reused
directly for `IRNode::Integer`/`Float`/`Str` — no new literal nodes needed
for those three.)

### Patterns

```text
SymPatternBlank { head: Option<Box<Expr>>, span }
    -- Wolfram `_` (head: None) or `_h` (head: Some(SymSymbol("h")))

SymPatternNamed { name: String, pattern: Box<Expr>, span }
    -- Wolfram `x_` desugars to SymPatternNamed { name: "x",
       pattern: SymPatternBlank { head: None } }; `x_h` to
       SymPatternNamed { name: "x", pattern: SymPatternBlank { head: Some(h) } }
```

### Rules and replacement

```text
SymRule { lhs: Box<Expr>, rhs: Box<Expr>, delayed: bool, span }
    -- delayed: false is `->` (Rule); true is `:>` (RuleDelayed)

SymReplaceAll {
    expr:     Box<Expr>,
    rules:    Vec<Expr>,   -- each element is a SymRule (or a SymApply
                              evaluating to a list of rules — the frontend
                              may leave this as a runtime concern)
    repeated: bool,        -- false: `/.` one pass; true: `//.` fixed point
    span,
}
```

## Matcher semantics (binding contract every backend must honor)

A backend implementing `SymReplaceAll` must:

1. Walk `expr`'s tree in the same traversal order `cas-pattern-matching`
   uses today — **bottom-up (post-order)**: a node's `head` and every
   `args` element are visited (and possibly replaced) before the node
   itself is tried against any rule. (An earlier draft of this bullet said
   "top-down, left-to-right over `args`" — that never matched what
   `cas-pattern-matching`'s `rewrite()` actually does; corrected here to
   match the real, tested algorithm this spec's own "port, not a
   reimplementation" framing below commits to.)
2. At each subtree, try each `rules[i].lhs` in order; the first structural
   match wins (no backtracking across rules — matches the reference CAS
   behavior).
3. A `SymPatternBlank { head: None }` matches any subtree; `head: Some(h)`
   matches only a `SymApply` whose `head` structurally equals `h` (or a bare
   `SymSymbol`/literal whose "head" is its own type tag, matching Wolfram's
   `Head[]` convention).
4. A `SymPatternNamed` binds `name` to the matched subtree for the rest of
   that match attempt; a second occurrence of the same `name` elsewhere in
   `lhs` requires structural equality with the first binding (not just any
   match) — this is the one place a matcher needs backtracking-free
   consistency checking, not full unification.
5. On match, substitute bound names into `rhs` and replace the subtree.
   `delayed: false` rules substitute into an already-evaluated `rhs` (built
   once, at rule-construction time, per Wolfram `Rule` semantics); `delayed:
   true` rules re-run substitution fresh per match (`RuleDelayed`).
6. `repeated: true` reruns the whole pass until no rule fires (a fixed
   point) or a backend-defined iteration cap is hit — every backend **must**
   enforce a cap here; an unbounded `//.` is a guaranteed non-terminating
   program for some inputs and every runtime in this repo enforces DoS caps
   on unbounded constructs (see `Range`/list-op caps already used by the
   native Wolfram runtime, `MA04` §10.3/§13.3/§19.5, for the established
   convention this must match).

This contract is deliberately identical to `cas-pattern-matching`'s existing
`match_pattern`/`rewrite` algorithm — `sir-runtime-symbolic` is a **port**,
not a reimplementation from a blank slate.

## New `Feature` flags

```text
Feature::SymbolicExpr
Feature::PatternMatching
Feature::Rationals    -- shared with SIR22
Feature::Complex      -- shared with SIR22
```

## Effects

`SymApply`, `SymRule`, `SymReplaceAll` are all `Pure` — pattern matching and
substitution are deterministic and side-effect-free. (A CAS session-level
side effect, like Macsyma's `assume`/`kill`, stays entirely inside each
frontend's own runtime and is out of scope for the IR — same "session state
isn't SIR" boundary the native runtimes already draw.)

## Backend impact

- **JS/TS**: new `match` arms construct/consume a tagged term-tree value at
  runtime via `sir-runtime-symbolic` (`__SirSym.apply(head, args)`,
  `__SirSym.replaceAll(expr, rules)`, `__SirSym.replaceRepeated(expr, rules,
  cap)`), imported only when `Feature::SymbolicExpr`/`PatternMatching` is in
  the manifest.
- **Rust/Go/Python backends**: not required in this first wave; they reject
  modules declaring `SymbolicExpr`/`PatternMatching` per the existing
  capability-rejection path.

## Versioning

Additive extension, same discipline as SIR16/SIR18/KW1/SIR22. Modules using
SIR23 nodes bump `metadata.sir_version`.

## References

Internal: [`HML01`](HML01-math-to-semantic-ir.md),
[`SIR10`](SIR10-narrow-waist-semantic-ir.md),
[`SIR22`](SIR22-array-matrix-semantic-ir.md) (sibling domain, shared
`Rational`/`Complex`), `symbolic-ir` (the `IRNode` shape this spec mirrors
1:1), `cas-pattern-matching` (the matcher algorithm `sir-runtime-symbolic`
ports), [`MA04`](MA04-wolfram-language.md) (the frontend that will emit
these nodes; also the source of the deferred sequence-pattern scope list).
