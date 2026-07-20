# Changelog

## [0.1.0] - 2026-07-19

### Added

- Initial `derive-to-semantic-ir` frontend crate (HML01 Stream B), the
  **third** to target SIR23 (symbolic-expression/pattern domain), sibling
  to `wolfram-to-semantic-ir` and `macsyma-to-semantic-ir`:
  `compile`/`compile_source` lowering `coding-adventures-derive-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`.
- Design: retargets `derive-runtime`'s own `lower_node` rule-name dispatch
  (which already lowers this exact CST to `symbolic_ir::IRNode`) onto
  `semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`) instead —
  every construct (arithmetic, comparisons, logic, vectors/matrices,
  function application, `:=` assignment/definition) lowers to symbolic
  data, matching `symbolic_ir::IRNode`'s "everything is one apply-tree"
  design.
- Covers all of `derive-parser`'s currently-implemented grammar
  productions (`program`/`statement_line`/`assignment`/`logical_or`/
  `logical_and`/`logical_not`/`comparison`/`additive`/`multiplicative`/
  `unary`/`power`/`postfix`/`atom`/`vector`/`row`/`group`/`arglist`).
- **Scope boundary, disclosed from day one, verified empirically against
  `code/grammars/derive/derive.grammar` and `derive.tokens`** (not just
  trusted from `derive-runtime`'s own doc comment, per this repo's
  verify-before-implementing discipline): Derive's grammar has no
  pattern-matching or rewrite-rule syntax at all (no `_`/blank, no `x_`
  named pattern, no `->`/`:>` rule arrow, no `/.`/`//.` replacement, and no
  `STRING` token at all) — this crate therefore only ever constructs
  `Expr::SymSymbol`/`Expr::SymApply` (plus reused `IntLit`/`FloatLit`),
  never `Expr::StrLit`, `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
  `SymReplaceAll`, and never observes `Feature::PatternMatching`. This also
  means `measure_depth_iterative`/`drop_iterative` only need a match arm
  for `Expr::SymApply`.
- Derive has **no control-flow grammar productions at all** (no
  `if`/`while`/`for`/`block`/`return` rules, unlike Macsyma) — `IF(…)` is
  an ordinary UPPERCASE builtin call bridged through the same
  surface→canonical table as `SIN`/`DIF`/`INT`, not a special grammar
  form. This crate therefore needs none of `macsyma-to-semantic-ir`'s
  synthetic `WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/`RETURN_HEAD` local
  constants, and imports `IF` directly from `symbolic_ir` (which already
  exports it, unlike those Macsyma-only synthetic heads).
- `standard_function` mirrors `derive-runtime::lower::surface_head_to_ir`'s
  table exactly — a BIGGER surface→canonical bridge than Wolfram's needs,
  since Derive's built-ins are conventionally UPPERCASE (`SIN`, `DIF`,
  `INT`) and `SymSymbol` equality is case-sensitive, so every
  elementary/hyperbolic function needs an explicit entry, not just the
  ones that differ semantically. `LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are
  deliberately absent, matching `derive-runtime`'s own disclosed "honest
  scope" (MA07 §4) — the shared VM/IR has no existing canonical head for
  them yet, so wiring them here would be new-head invention, not reuse.
- `F(x, y) := body` (LHS lowers to `SymApply{head: SymSymbol(_), ..}`)
  lowers to `Define(F, List(x, y), body)`; a bare `x := body` lowers to
  `Assign(x, body)` — disambiguated purely by the lowered LHS's shape,
  since Derive's grammar has exactly ONE assignment token (`:=`), unlike
  Wolfram's `=`/`:=` or Macsyma's `:`/`:=` pairs. Mirrors
  `derive-runtime::lower::lower_assignment`'s identical logic exactly.
- `[a, b, c]` / `[a, b; c, d]` vector/matrix literals lower to structural
  `List` data by counting parsed `row` children (one row → flat
  `List(elems…)`; more than one → `List` of per-row `List`s) — mirrors
  `derive-runtime::lower::lower_vector`'s identical logic. Structural only,
  no linear-algebra evaluation wired here (separate, later work).
- Recursion-depth hardening applied from day one, proactively carried over
  from `wolfram-to-semantic-ir`'s and `macsyma-to-semantic-ir`'s own
  security-review history, even though neither `derive-parser` nor
  `derive-runtime` (the retarget source) applies any of these guards
  themselves:
  - `MAX_EXPR_DEPTH` (256), the same value and reasoning as the sibling
    SIR23 frontends — `derive-parser`'s own measured bare-stack crash
    floor (298 `parse_rule` frames) is even higher than
    `macsyma-parser`'s/`wolfram-parser`'s (~275-278), so 256 remains a
    conservative, consistent value to reuse rather than inventing a new
    one.
  - `check_chain_length` caps every flat operator-chain fold (`additive`/
    `multiplicative`/`logical_or`/`logical_and`) before any tree is built.
  - `check_postfix_chain_length` caps chained call application
    (`F(x)(y)(z)…`) — like Macsyma's (and simpler than Wolfram's
    cumulative-budget variant), Derive's `postfix` has only one suffix
    shape, so a plain per-chain group count is already an exact bound.
  - `check_apply_arg_count` caps `arglist` argument counts **and**
    vector/matrix row and per-row element counts — a defense-in-depth
    allocation-size backstop unique in scope to this crate among the
    SIR23 frontends so far, since Derive's `vector` grammar rule (D-5) has
    no analogue in Wolfram's or Macsyma's currently-implemented grammars.
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement, closing the gap
    that per-construct guards don't compose across nested `(...)`
    boundaries.
- `Feature::Floats` regression avoided proactively: `number_literal_expr`
  is an instance method (not a free function) specifically so every
  `FloatLit`-constructing branch can call `self.observed.add(Feature::
  Floats)` immediately — this is a confirmed, previously-shipped bug in
  both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir`.
- `compile_source` needs no worker-thread stack enlargement, unlike
  `wolfram-to-semantic-ir::compile_source`: `derive-parser`'s own
  `MAX_RULE_DEPTH` (200) is already documented safe on a bare default
  (~2 MiB) stack with comfortable margin — mirrors
  `macsyma-to-semantic-ir`'s simpler `compile_source` shape.
- `tests/e2e_node.rs` — written directly against the *current* SIR23 JS
  backend state (real `__Sir.Symbolic.*` codegen, confirmed by reading
  `macsyma-to-semantic-ir`'s and `wolfram-to-semantic-ir`'s current test
  bodies, not their module doc comments — `macsyma-to-semantic-ir`'s own
  doc comment and README were stale on exactly this point until a
  follow-up fix) — compiles and runs representative Derive programs
  (arithmetic, function definition+call, assignment, UPPERCASE builtin
  calls, vectors/matrices, a multi-statement program) through `node`,
  proving the SIR23 codegen path is genuinely executable, not just
  statically accepted. Unlike `macsyma-to-semantic-ir`'s initial ship
  (which had no e2e node test at all, since it predated the SIR23 codegen
  landing), this crate ships `e2e_node.rs` from v0.1.0.
- 58 tests: 44 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (flat-chain, chained call
  application, a wide vector literal, deeply nested parens, and
  exact-boundary cases) and the `Feature::Floats` regression, 7
  validator/capability-acceptance tests, 6 e2e `node`-execution tests
  (skip cleanly if `node` is unavailable), 1 doctest.
- Adds `derive-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s
  workspace `members` and marks it done in
  `HML01-math-to-semantic-ir.md`'s Stream B rollout.
