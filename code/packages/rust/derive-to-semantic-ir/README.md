# derive-to-semantic-ir

Derive CST → narrow-waist Semantic IR. The **third** frontend to target
[SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
`wolfram-to-semantic-ir` (the first) and `macsyma-to-semantic-ir` (the
second).

## Where this fits

```
Derive source
   │
   ▼  coding_adventures_derive_parser::try_parse_derive(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  derive_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use derive_to_semantic_ir::compile_source;

let module = compile_source("1 + 2\n", "demo")?;
```

`compile_source` parses and lowers directly, with no worker-thread stack
enlargement — like `macsyma-to-semantic-ir` (and unlike
`wolfram-to-semantic-ir::compile_source`), `derive-parser`'s own
`MAX_RULE_DEPTH` (200) is already documented safe on a bare default
(~2 MiB) stack with comfortable margin (measured crash floor 298
`parse_rule` frames, even higher than `macsyma-parser`'s/`wolfram-parser`'s
own ~275-278). `compile` (taking an already-parsed `GrammarASTNode`) is
pure lowering, exactly like every sibling frontend's `compile`.

## Design: retargeting `derive-runtime`

`derive-runtime` already walks this exact CST and compiles it to
`symbolic_ir::IRNode` — this crate's dispatch table is a direct retarget
of that lowering's own rule-name dispatch onto `semantic_ir::Expr`'s
SIR23 vocabulary (`SymSymbol`/`SymApply`) instead. Every construct —
arithmetic, comparisons, vectors/matrices, function application, `:=`
assignment/definition — lowers to symbolic *data*, mirroring
`symbolic_ir::IRNode`'s "everything is one apply-tree" design. See
`src/lower.rs`'s module doc comment for the full reasoning and the
node-by-node mapping.

Derive is **much thinner** than Wolfram's or Macsyma's surface: no
`f[x]`-universal-application syntax (ordinary parens double as grouping
and application), and — per MA07 §1 — no control-flow grammar productions
at all (`IF(…)` is an ordinary UPPERCASE builtin call, bridged through the
same surface→canonical table as `SIN`/`DIF`/`INT`, not a special `if_expr`
grammar rule the way Macsyma has one). So this crate needs no synthetic
`WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/`RETURN_HEAD` local constants
the way `macsyma-to-semantic-ir` does, and can import `IF` directly from
`symbolic_ir` (which already exports it).

## Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax

**Verified empirically** against `code/grammars/derive/derive.grammar` and
`derive.tokens` (not just trusted from `derive-runtime`'s own doc
comment): neither file declares any pattern-syntax token or rule — no
`_`/blank, no `x_` named-pattern shape, no `->`/`:>` rule arrow, no
`/.`/`//.` replacement operators. This crate therefore **only ever
constructs** [`Expr::SymSymbol`] and [`Expr::SymApply`] (plus the reused
`IntLit`/`FloatLit` literal nodes — Derive has no `STRING` token at all,
so `Expr::StrLit` is never constructed either) — it never constructs
`SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`, and never
observes `Feature::PatternMatching`. This is a disclosed scope boundary
matching the grammar's actual surface, not an oversight — see
`src/lower.rs`'s module doc comment for the full discussion, including why
this simplifies the recursion-depth-hardening helpers
(`measure_depth_iterative`/`drop_iterative` only need a match arm for
`Expr::SymApply`).

### A bigger surface→canonical bridge than Wolfram's

Derive's built-ins are conventionally UPPERCASE (`SIN`, `DIF`, `INT`, `IF`
— MA07 §3), and `SymSymbol` equality is case-sensitive, so *every*
elementary/hyperbolic function and every renamed calculus/control head
needs an explicit bridge-table entry — not just the handful that differ
semantically the way Wolfram's bridge does. `standard_function` mirrors
`derive-runtime::lower::surface_head_to_ir`'s table exactly (same surface
names, same canonical `symbolic_ir` head constants), so the native-eval
and SIR23 lowerings can never drift apart on what a given builtin
canonicalizes to. `LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are deliberately
absent, matching `derive-runtime`'s own disclosed "honest scope" (MA07
§4) — the shared VM/IR has no existing canonical head for them yet.

### `:=` disambiguation has no operator to branch on

Derive's grammar has exactly ONE assignment token, `ASSIGN` (`:=`) — `x :=
5` and `F(x) := x^2 + 1` are syntactically identical until lowering.
`lower_assignment` disambiguates purely by the *lowered LHS's shape*:
`SymApply{head: SymSymbol(_), ..}` → `Define`, anything else → `Assign` —
exactly mirroring `derive-runtime::lower::lower_assignment`'s identical
logic.

### Vectors/matrices as structural `List` data (D-5)

`[a, b, c]` / `[a, b; c, d]` parse as a single `vector` grammar rule with
no separate "vector" vs "matrix" rule — `lower_vector` draws that
distinction purely by *counting* how many `row` children were parsed:
exactly one row → a flat `List(elems…)`; more than one → a `List` of
per-row `List`s. Structural only — no linear-algebra evaluation is wired
here (that is separate, later work), mirroring `derive-runtime::lower::
lower_vector`'s identical logic.

### Recursion-depth hardening

Carried over proactively from `wolfram-to-semantic-ir`'s (four rounds of
security review) and `macsyma-to-semantic-ir`'s established pattern, even
though neither `derive-parser` nor `derive-runtime` applies any of these
guards themselves — this is a `*-to-semantic-ir`-frontend-specific
defense, not part of the native pipeline:

- `MAX_EXPR_DEPTH` (256) bounds this crate's own lowering recursion,
  independent of `derive-parser`'s own grammar-nesting guard.
- `check_chain_length` caps every flat, same-precedence operator-chain
  fold (`additive`/`multiplicative`/`logical_or`/`logical_and`) before any
  tree is built, since Derive's grammar — like Wolfram's and Macsyma's —
  collapses a long unparenthesized chain into one CST node with many
  children rather than nesting through parens.
- `check_postfix_chain_length` caps chained call application
  (`F(x)(y)(z)…`) — like Macsyma's postfix (and unlike Wolfram's), Derive's
  `postfix` has only ONE suffix shape (a call), so a plain per-chain group
  count is already an exact bound.
- `check_apply_arg_count` caps `arglist` argument counts AND vector/matrix
  row and per-row element counts — flat-`Vec` allocation-size backstops,
  not stack guards (a construct unique to this crate among the SIR23
  frontends so far, since Derive's `vector` rule is new relative to
  Macsyma's list-only `[...]`).
- `measure_depth_iterative`/`drop_iterative` — the authoritative,
  construction-composition-independent iterative depth check and
  iterative teardown, run once per top-level statement, closing the gap
  that per-construct guards (each scoped to one grammar node) don't
  compose across nested `(...)` boundaries.

Also carried over proactively: every branch that constructs a `FloatLit`
calls `self.observed.add(Feature::Floats)` immediately — a confirmed,
previously-shipped bug in both `matlab-to-semantic-ir` and
`wolfram-to-semantic-ir` (their number-literal helpers were free functions
with no access to the feature-tracking state). This crate's
`number_literal_expr` is an instance method specifically so this can never
regress.

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production (arithmetic, comparisons, logic, assignment vs.
  definition disambiguation, vectors/matrices, UPPERCASE builtin bridging,
  lowercase-is-not-bridged), plus DoS-guard regression tests (flat operator
  chains, chained call application, a wide vector literal, all at ~3,000
  terms/groups/elements), exact-boundary tests (256/257), and the
  `Feature::Floats` regression test.
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate` (manifest declares exactly the SIR23 features
  used, and never `Feature::PatternMatching`) and is **accepted** by
  `semantic-ir-to-javascript`'s capability check — SIR23 JS codegen has
  been implemented since HML01 Stream B rollout item 7, confirmed by
  reading `macsyma-to-semantic-ir`'s and `wolfram-to-semantic-ir`'s
  *current* test bodies directly rather than trusting either crate's
  (now-stale, in `macsyma-to-semantic-ir`'s case) module doc comment.
- `tests/e2e_node.rs` — compiles and runs representative Derive programs
  (arithmetic, a function definition+call, assignment, UPPERCASE builtin
  calls, vectors/matrices, a multi-statement program) through `node`,
  proving the SIR23 codegen path is genuinely executable end-to-end, not
  just statically accepted.
