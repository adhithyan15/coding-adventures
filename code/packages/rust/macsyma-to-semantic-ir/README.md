# macsyma-to-semantic-ir

Macsyma CST → narrow-waist Semantic IR. The **second** frontend to target
[SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
symbolic-expression/pattern-matching domain extension of the SIR10
narrow-waist IR (Stream B of
[HML01](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
`wolfram-to-semantic-ir`, the first.

## Where this fits

```
Macsyma source
   │
   ▼  coding_adventures_macsyma_parser::create_macsyma_parser(src).parse()
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  macsyma_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR23)
```

## Usage

```rust
use macsyma_to_semantic_ir::compile_source;

let module = compile_source("1 + 2$\n", "demo")?;
```

`compile_source` parses and lowers directly, with no worker-thread stack
enlargement — unlike `wolfram-to-semantic-ir::compile_source`,
`macsyma-parser`'s own `MAX_RULE_DEPTH` (200) is already documented safe on
a bare default (~2 MiB) stack with comfortable margin, so this crate
mirrors `matlab-to-semantic-ir`'s and `apl-to-semantic-ir`'s simpler
`compile_source` shape instead of `wolfram-to-semantic-ir`'s worker-thread
pattern. `compile` (taking an already-parsed `GrammarASTNode`) is pure
lowering, exactly like every sibling frontend's `compile`.

## Design: retargeting `macsyma-compiler`

`macsyma-compiler` already walks this exact CST and compiles it to
`symbolic_ir::IRNode` — this crate's dispatch table is a direct retarget of
that compiler's own `Compiler::compile_node` rule-name dispatch onto
`semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`) instead.
Every construct — arithmetic, comparisons, lists, function application,
`:`/`:=` assignment/definition, and every control-flow form (`if`/`elseif`/
`else`, `while`, `for`, `block`, `return`) — lowers to symbolic *data*,
mirroring `symbolic_ir::IRNode`'s "everything is one apply-tree" design and
Macsyma's own native runtime, which interprets that data directly. See
`src/lower.rs`'s module doc comment for the full reasoning and the
node-by-node mapping.

## Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax

Unlike Wolfram, Macsyma's currently-implemented grammar (all 24 rules in
`macsyma-parser`) has **no** pattern-matching or rewrite-rule surface syntax
at all: no `_`/blank, no `x_` named-pattern shape, no `->`/`:>` rule arrow
(the lexer tokenizes `ARROW` but no parser rule consumes it), no `/.`/`//.`
replacement operators. This crate therefore **only ever constructs**
[`Expr::SymSymbol`] and [`Expr::SymApply`] (plus the reused `IntLit`/
`FloatLit`/`StrLit` literal nodes) — it never constructs
`SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`, and never
observes `Feature::PatternMatching`. This is a disclosed scope boundary
matching the grammar's actual surface, not an oversight — see
`src/lower.rs`'s module doc comment for the full discussion, including why
this simplifies the recursion-depth-hardening helpers
(`measure_depth_iterative`/`drop_iterative` only need a match arm for
`Expr::SymApply`).

### Recursion-depth hardening

Every flat, same-precedence operator chain (`additive`/`multiplicative`/
`logical_or`/`logical_and`) is capped at `MAX_EXPR_DEPTH` (256) operands
before any tree is built, because Macsyma's grammar — like Wolfram's and
MATLAB's — collapses a long unparenthesized chain into one CST node with
many children rather than nesting through parens, so it never trips the
ordinary grammar-nesting depth guard. Chained call application
(`f(x)(y)(z)…`) gets an analogous but simpler cap than Wolfram's postfix
guard: Macsyma's `postfix` has only one suffix shape (a call), so a plain
per-chain count of call groups is already an exact bound — there is no
second suffix shape (like Wolfram's `[[…]]` Part indexing) that multiplies
against it, so no cumulative budget is needed.

A **new** DoS-risk class not present in Wolfram's grammar at all: Macsyma's
`if_expr` production folds a flat `{ elseif expr then expr }` repetition —
one CST node, cheap to parse regardless of clause count — into a *nested*
chain of `If` `SymApply`s, one level per clause. `check_if_chain_length`
rejects the clause count before folding, exactly mirroring the flat-chain
guards' own reasoning.

The authoritative, construction-composition-independent check is
[`measure_depth_iterative`](src/lower.rs) — an iterative (never recursive)
post-construction depth check, safe to call on a tree of any size because
building a deeply-nested `Box`-based tree only costs heap, not stack. It
runs once per top-level statement before anything reaches the returned
`Module`. [`drop_iterative`](src/lower.rs) tears down a rejected tree the
same way, using an explicit work stack rather than `Expr`'s ordinary
recursive `Drop` glue — this hardening, and the reasoning behind every
guard above, was carried over proactively from `wolfram-to-semantic-ir`'s
own four-round security-review history (see that crate's `CHANGELOG.md`)
rather than discovered fresh here.

Also carried over proactively: every branch that constructs a `FloatLit`
calls `self.observed.add(Feature::Floats)` immediately — a confirmed,
previously-shipped bug in both `matlab-to-semantic-ir` and
`wolfram-to-semantic-ir` (their number-literal helpers were free functions
with no access to the feature-tracking state, so a float-literal-only
module failed `semantic_ir::validate()`). This crate's `number_literal_expr`
is an instance method specifically so this can never regress.

### Testing

- `tests/test_lower.rs` — unit tests asserting exact `Expr` shapes for
  every grammar production, plus DoS-guard regression tests (flat operator
  chains, chained call application, and the if/elseif chain, all at ~3,000
  terms/groups/clauses — comfortably past the cap without the parse-time
  slowness that made `wolfram-to-semantic-ir` rescale its own 60,000-term
  tests down), exact-boundary tests (256/257), and the `Feature::Floats`
  regression test.
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate` (manifest declares exactly the SIR23 features
  used) and is correctly **accepted** by `semantic-ir-to-javascript`'s
  capability check (`sir-runtime-symbolic`'s SIR23 codegen has since
  landed — see the crate `CHANGELOG.md`'s `[Unreleased]`/`[0.1.1]` "Fixed"
  entry; this file's assertions used to say the opposite, which was itself
  a stale-test bug).
- `tests/e2e_node.rs` — actually compiles and runs representative Macsyma
  programs (arithmetic, a function definition+call, assignment,
  control-flow constructs, a multi-statement program) through `node`,
  proving the SIR23 codegen path is genuinely executable, not just
  statically accepted. Asserts only that `node` exits cleanly — no
  `disp`-equivalent stdout to diff against, under this crate's own
  "everything is symbolic data" design.
- `tests/oracle.rs` — HML01 §7 oracle/golden testing (a 34-case corpus):
  the SAME Macsyma source run through **two independent
  implementations** — `macsyma-runtime` (ground truth) and this crate's
  own `compile_source` → `semantic_ir_to_javascript::compile` → `node` —
  and diffed. Modeled directly on `wolfram-to-semantic-ir/tests/oracle.rs`.
  Covers arithmetic/identity-law folding, every comparison and logic
  operator (Macsyma's own spelling: `=`/`#`/`and`/`or`/`not`), elementary
  functions, `:` (`Assign`) and `:=` (`Define`) — including a
  genuinely-working function-definition-and-call round-trip, unlike
  Wolfram's own frontend, since Macsyma's `:=` lowering already produces
  the exact 3-argument `Define` shape the shared JS handler requires —
  `if`/`then`/`else`, list-literal evaluation (`known_bug`'d for the
  missing bracket display convention), and two control-flow-as-data cases
  that surprisingly match verbatim on both sides despite being
  non-atomic. Also documents (in the file's own module doc) why a
  separate Maxima oracle corpus is unnecessary duplication:
  `maxima-to-semantic-ir` is a pure re-export of this crate with no shim,
  and `maxima-runtime` is a thin façade over this crate's own sibling
  `macsyma-runtime`, formatting the identical `output_text`.
