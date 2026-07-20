# Changelog

## [0.1.0] - 2026-07-19

### Added

- Initial `reduce-to-semantic-ir` frontend crate (HML01 Stream B), the
  **fourth** to target SIR23 (symbolic-expression/pattern domain), sibling
  to `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, and
  `derive-to-semantic-ir`: `compile`/`compile_source` lowering
  `coding-adventures-reduce-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`.
- Design: retargets `reduce-runtime`'s own `lower_node` rule-name dispatch
  (which already lowers this exact CST to `symbolic_ir::IRNode`) onto
  `semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`) instead —
  much of the shape is a direct copy of `derive-to-semantic-ir`'s own
  lowering (`reduce-runtime`'s own module doc comment says so explicitly),
  since Reduce, like Derive, has no `f[x]`-universal-application syntax and
  no pattern/rewrite-rule vocabulary in this subset (MA08 §4 defers `let`
  rules).
- Covers all of `reduce-parser`'s currently-implemented grammar
  productions (`program`/`statement_line`/`if_expr`/`group_expr`/
  `assignment`/`logical_or`/`logical_and`/`logical_not`/`comparison`/
  `cons`/`additive`/`multiplicative`/`unary`/`power`/`postfix`/`atom`/
  `list_literal`/`group`/`arglist`).
- **Scope boundary, disclosed from day one, verified empirically against
  `code/grammars/reduce/reduce.grammar` and `reduce.tokens`** (not just
  trusted from `reduce-runtime`'s own doc comment, per this repo's
  verify-before-implementing discipline): Reduce's grammar has no
  pattern-matching or rewrite-rule syntax at all (no `_`/blank, no `x_`
  named pattern, no `->`/`:>` rule arrow, no `/.`/`//.` replacement, and no
  `STRING` token at all) — this crate therefore only ever constructs
  `Expr::SymSymbol`/`Expr::SymApply` (plus reused `IntLit`/`FloatLit`),
  never `Expr::StrLit`, `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
  `SymReplaceAll`, and never observes `Feature::PatternMatching`. This also
  means `measure_depth_iterative`/`drop_iterative` only need a match arm
  for `Expr::SymApply` — `If`/`CompoundExpression`/`Cons` are all
  `SymApply` with a different head symbol, not new `Expr` variants.
- **Three genuinely new constructs beyond Derive's grammar**, each
  retargeting `reduce-runtime::lower`'s own identical logic:
  - An expression-shaped `if` (`if_expr = "if" expr "then" expr [ "else"
    expr ]`) — lowers to `If(cond, then[, else])`, usable anywhere an
    `expr` can appear (including as a `:=` right-hand side), unlike
    anything in Derive's grammar.
  - A group statement `<< s1; s2; ... >>` (`group_expr`) — lowers to
    `CompoundExpression(s1, s2, ...)`.
  - Cons (`a . b`, `cons = additive [ DOT cons ]`) — `fold_cons` folds a
    cons onto a structurally literal `List` RHS directly into one flat
    `List` (the one shape MA08 §3 documents); a non-list RHS lowers to a
    bare `Cons(a, b)` head, a disclosed, documented gap mirroring
    `reduce-runtime::lower::fold_cons` exactly.
  - Lists (`{a, b, c}`, curly braces per MA08 §1/§3) are always flat (no
    row/matrix shape — matrices are out of Reduce's scope, MA08 §4), so
    `lower_list_literal` reuses `lower_arglist` directly instead of
    `derive-to-semantic-ir::lower_vector`'s row-counting split.
- **Confirmed and reused a REAL divergence from MA08 §3's own prose**:
  the spec's table spells arithmetic's "Lowers to" column as `Plus`/
  `Subtract`/`Times`/`Power`, even describing `a/b` expanding to
  `Times[a, Power[b,-1]]` and `-a` to `Times[-1,a]` — **none of those
  spellings exist in `symbolic-ir`**, confirmed directly
  (`grep -n '"Plus"\|"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs`
  returns nothing). The REAL heads — what `symbolic_vm::handlers::
  build_handler_table` actually wires, and what `reduce-runtime::lower`
  itself already uses — are `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`, the exact
  same heads `derive-to-semantic-ir`/`macsyma-to-semantic-ir` use. This
  crate reuses those same real heads (the identical `symbolic_ir`
  constants), NOT new `Plus`/`Subtract`/`Times`/`Power` string literals —
  a disclosed, deliberate divergence from the spec's literal prose
  (already corrected in MA08's own changelog-style note), not new-head
  invention.
- **Confirmed and reused a REAL gap**: `CompoundExpression`, `First`,
  `Second`, `Third`, `Rest`, `Part`, `Append`, `Reverse` (and a non-folding
  `Cons`) have no evaluation handler in the shared `symbolic_vm::handlers::
  build_handler_table` — `reduce-runtime` reuses the shared backend
  unchanged rather than building a bespoke one, per its own design
  mandate. Largely moot for this crate (it never evaluates anything, per
  the "everything is data" design every SIR23 frontend shares) — confirmed
  directly by reading `semantic-ir-to-javascript/src/emit.rs`'s SIR23
  codegen, which lowers `Expr::SymApply` to
  `__Sir.Symbolic.apply(head, [args...])` uniformly for ANY head spelling,
  with no per-head special-casing. This crate reuses the exact head
  spellings `reduce-runtime` uses for these, via its own locally-defined
  `pub const`s (`COMPOUND_EXPRESSION`, `CONS`, `FIRST`, `SECOND`, `THIRD`,
  `REST`, `PART`, `APPEND`, `REVERSE` — not exported by `symbolic-ir`, and
  this crate does not depend on `reduce-runtime` itself, mirroring the
  same "locally-defined pub const, spelled to match a sibling crate's
  constant" pattern `macsyma-to-semantic-ir` needed for its own
  `WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/`RETURN_HEAD` constants).
- `h(l, m) := body` (LHS lowers to `SymApply{head: SymSymbol(_), ..}`)
  lowers to `Define(h, List(l, m), body)`; a bare `x := body` lowers to
  `Assign(x, body)` — disambiguated purely by the lowered LHS's shape,
  since Reduce's grammar has exactly ONE assignment token (`:=`). Unlike
  Derive's self-referential `assignment = logical_or [ ASSIGN assignment
  ]`, Reduce's right-hand side is the WIDER `expr` production (a
  grammar-level divergence `reduce.grammar`'s own comment discloses, not
  an oversight this crate works around) — `x := if a>0 then 1 else -1`
  and `x := << a:=1; a+1 >>` both parse and lower directly through the
  same `if_expr`/`group_expr` dispatch, no special-casing needed.
- Reduce's `neq` (a `KEYWORD`-typed token, matched by literal value
  alongside the four symbolic comparison token *types*) lowers to
  `NotEqual` — a comparison Derive's grammar has no equivalent token for
  at all.
- Recursion-depth hardening applied from day one, proactively carried over
  from `wolfram-to-semantic-ir`'s, `macsyma-to-semantic-ir`'s, and
  `derive-to-semantic-ir`'s own security-review history, even though
  neither `reduce-parser` nor `reduce-runtime` (the retarget source)
  applies any of these guards themselves:
  - `MAX_EXPR_DEPTH` (256), the same value and reasoning as the sibling
    SIR23 frontends, kept for family-wide consistency even though
    `reduce-parser`'s own cap (128) is lower than `derive-parser`'s (200)
    — this constant bounds a different axis (this crate's own
    chain-folding budget) than the parser's CST-nesting cap.
  - `check_chain_length` caps every flat operator-chain fold (`additive`/
    `multiplicative`/`logical_or`/`logical_and`) before any tree is built
    — confirmed these ARE flat EBNF repetitions (not right-recursion)
    directly against `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment,
    which measured an uncapped parser accepting one million repeated
    items with zero crashes for exactly this shape.
  - `check_postfix_chain_length` caps chained call application
    (`f(x)(y)(z)…`) — like Derive's postfix (and simpler than Wolfram's
    cumulative-budget variant), Reduce's `postfix` has only one suffix
    shape, so a plain per-chain group count is already an exact bound.
  - `check_apply_arg_count` caps `arglist`/`list_literal` element counts
    **and** `group_expr`'s flat `{ (SEMI|DOLLAR) expr }` statement-
    sequence length — a defense-in-depth allocation-size backstop, mirrors
    `derive-to-semantic-ir`'s identical reuse of this one guard across
    `arglist` and vector-row counts.
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement.
  - **No additional lowering-side guard is needed** for Reduce's five
    genuinely self-referential (right-recursive) productions —
    parenthesised nesting, the `:=` chain, the `if`/`else` chain, the cons
    (`.`) chain, and the power (`^`) chain — since `reduce-parser`'s own
    `MAX_RULE_DEPTH` (128; binding constraint measured at a 179-rule-frame
    cons-chain floor, ~28.5% margin) already bounds how deep any of these
    can nest in the CST this crate ever receives. Verified with dedicated
    regression tests: a 5,000-level deep cons chain and a 5,000-level deep
    `if`/`else` chain are both cleanly rejected (by the parser, surfaced
    as a clean `Err` through `compile_source`), never crash.
- `Feature::Floats` regression avoided proactively: `number_literal_expr`
  is an instance method (not a free function) specifically so every
  `FloatLit`-constructing branch can call `self.observed.add(Feature::
  Floats)` immediately — this is a confirmed, previously-shipped bug in
  both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir`.
- `compile_source` needs no worker-thread stack enlargement, unlike
  `wolfram-to-semantic-ir::compile_source`: `reduce-parser`'s own
  `MAX_RULE_DEPTH` (128) is already documented safe on a bare default
  (~2 MiB) stack with comfortable margin (28.5% below its own measured
  crash floor) — mirrors `macsyma-to-semantic-ir`'s/`derive-to-semantic-
  ir`'s simpler `compile_source` shape.
- `tests/e2e_node.rs` — written directly against the *current* SIR23 JS
  backend state (real `__Sir.Symbolic.*` codegen, confirmed by reading
  `derive-to-semantic-ir`'s current test bodies, not module doc comments)
  — compiles and runs representative Reduce programs (arithmetic, a
  procedure definition+call, assignment, list accessor calls, lists/cons,
  `if` expressions, a group statement, a multi-statement program) through
  `node`, including constructs with no shared-VM evaluation handler
  (`CompoundExpression`, list accessors, a non-folding `Cons`) — proving
  the SIR23 codegen path accepts and executes them as pure data
  construction regardless of runtime evaluability.
- 78 tests: 59 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (flat-chain, chained call
  application, a wide list literal, a wide group statement, a deeply
  parenthesised expression, a deep cons chain, a deep `if`/`else` chain,
  and exact-boundary cases at 256/257) and the `Feature::Floats`
  regression, 10 validator/capability-acceptance tests (including the two
  no-shared-VM-handler constructs), 8 e2e `node`-execution tests (skip
  cleanly if `node` is unavailable), 1 doctest.
- Adds `reduce-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s
  workspace `members` and marks it done in
  `HML01-math-to-semantic-ir.md`'s language list and Stream B rollout
  note, closing the `reduce-to-semantic-ir` gap that note previously
  called out as an open follow-on item (only `maple-to-semantic-ir`
  remains open now).
