# Changelog

## [0.1.1] - 2026-07-21

### Added

- **`tests/oracle.rs` — HML01 §7 oracle/golden testing, cross-checking
  `derive-runtime` (ground truth) against `derive_to_semantic_ir::
  compile_source` → `semantic_ir::Module` → `semantic_ir_to_javascript::
  compile` → a real `node` process.** The direct Derive sibling of
  `j-to-semantic-ir/tests/oracle.rs`, completing HML01 §5's "a true oracle
  diff [for Stream B] remains the one open item" note *for
  `derive-to-semantic-ir` specifically* (that spec line is updated by this
  PR; `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`/`reduce-to-
  semantic-ir`/`maple-to-semantic-ir` are unaffected and still have no
  oracle file of their own). 38-case corpus: bare integer/float/symbol
  atoms; ordinary (non-J/APL) operator precedence and right-associative
  `^`; unary minus binding looser than `^`; exact-integer vs. genuine-
  rational division; an additive-identity simplification; assignment and
  vector-assignment read back by a later statement; single- and
  multi-parameter user-defined function definition/call; `DIF`/`INT` via
  the shared calculus handlers (including the differentiate-then-call-at-
  a-point worksheet idiom `derive-runtime`'s own test suite uses); `IF`'s
  two branches; every comparison/logic keyword (`= <= < > >= AND OR NOT`,
  including a 3-term `AND` chain exercising the n-ary logical-chain fold);
  flat/singleton/elementwise-evaluated vectors and 2×2/3-row-1-column
  matrices (D-5 structural `List` data).
- Adds a dev-dependency on `coding-adventures-derive-runtime` (this
  frontend's own sibling native-runtime crate) for `tests/oracle.rs`'s
  ground truth only — the non-dev `[dependencies]` section still does not
  depend on it; lowering itself only ever needs the parse-tree shape.

### Found, NOT fixed here (shared `semantic-ir-to-javascript` crate — follow-up task)

Building this corpus found that comparing *evaluated values* — the entire
point of an oracle test — is currently blocked for this frontend (and, by
the same root cause, for `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`,
Stream B's other two shipped frontends, which is presumably why neither
has ever shipped an oracle file either) by two gaps in the SHARED
`semantic-ir-to-javascript` crate, not in this frontend's own lowering
(confirmed independently: `tests/test_lower.rs`'s ~40 pre-existing shape
assertions all still pass unmodified, so `derive_to_semantic_ir` itself
emits exactly the `SymApply`/`SymSymbol` shapes MA07 §3 calls for).
Recorded here — mirroring how `j-to-semantic-ir`'s own oracle file
documented its two shared-crate bugs excluded-not-fixed — rather than
patched in this PR, per this task's own scope discipline (`tests/
oracle.rs`'s module doc has the full write-up with confirmed emitted-JS
examples for each):

- **No SIR23 evaluation or simplification of any kind.** `Expr::SymApply`
  compiles unconditionally to `__Sir.Symbolic.apply(head, [args])` — a
  pure, inert term constructor, confirmed by hand-compiling and running
  representative programs through `node`: `1 + 2*3` stays `Add(1, Mul(2,
  3))` (never folds to `7`); `x := 5` / `x + 1` compiles to `Assign(x, 5)`
  then `Add(x, 1)` (the second statement's `x` is never substituted, so it
  never reads back `6`); `DIF(x^2, x)` stays `D(Pow(x, 2), x)` (never
  differentiates to `2*x`); `5 > 3` stays `Greater(5, 3)` (never evaluates
  to the symbol `True`); `F(x) := x*x` / `F(5)` never registers `F` or
  dispatches the call. The SIR23 domain's `SymReplaceAll`/
  `SymReplaceRepeated` nodes DO have a real pattern-rewrite implementation
  (`runtime.rs`'s `Symbolic.replaceAll`/`replaceRepeated`), but Derive's
  grammar has no rewrite-rule syntax at all (MA07 §4) to ever emit those
  nodes, so this frontend can never reach that machinery either. 29 of
  this crate's 38 new oracle cases hit this gap (every case beyond a bare
  literal/symbol atom and beyond the 5 display-only cases below).
- **No per-source-language SIR23 display convention.** Even a term that
  WAS already fully reduced would still print wrong: the sole SIR23
  stringifier, `Symbolic.toDisplayString`, renders every compound term
  generically as `head(args, ...)` — `Add(x, 1)`, `List(1, 2, 3)`,
  `Neg(x)` — with no infix `+`/`*`/`^`, no `[...]`/`[...;...]` bracket
  convention, no prefix `-`/`NOT`, and no case-bridging back to Derive's
  own UPPERCASE builtin spelling (`derive-runtime::printer::print_derive`
  reverses all four). Unlike the SIR22 array domain's `ArrayRt.fmtNum`/
  `display` (which already has per-language flags —
  `SIR_DISPLAY_APL_HIGH_MINUS`, `SIR_DISPLAY_J_UNDERSCORE`), the SIR23
  domain has no such mechanism for any source language yet. 5 of this
  crate's 38 new oracle cases (`equation_with_a_free_variable_stays_
  symbolic`, `flat_vector_literal`, `singleton_vector_literal`,
  `two_by_two_matrix_literal`, `three_row_one_column_matrix_literal`) hit
  ONLY this gap — the value needs no evaluation at all, but the printed
  notation still disagrees.

### Known limitations

- **No local lowering bugs were found in this pass** (unlike
  `j-to-semantic-ir`'s oracle PR, which found and fixed two bugs genuinely
  local to that frontend's own `src/lower.rs`). This crate's lowering was
  already independently verified, node-by-node, against
  `derive-runtime::lower`'s identical dispatch table (see this crate's
  0.1.0 entry above), and `tests/test_lower.rs`'s ~40 shape-assertion
  tests already cover every grammar production directly — an oracle test
  that can only ever compare unevaluated term SHAPES (per the two gaps
  above) cannot surface a new class of bug beyond what those direct shape
  assertions already check.
- `tests/oracle.rs` performs one test-local transformation neither
  `j-to-semantic-ir`'s nor `apl-to-semantic-ir`'s own oracle file needed:
  after `compile_source` + `semantic_ir::validate` (so validation still
  exercises exactly what shipped, unmodified), it wraps each top-level
  statement's `Expr` in `BuiltinCall("print", [expr])` using only
  `semantic_ir`'s own public `Module`/`Stmt`/`Expr` types, purely so a
  value is observable on the compiled side at all. `derive_to_semantic_ir
  ::compile_source` itself is intentionally unchanged and still emits no
  `print`/`console.log` of its own for any other caller — `tests/
  e2e_node.rs`'s own module doc comment's "no `disp`-equivalent stdout"
  design note is still accurate for the crate's real, shipped behavior.
- Given the above, only 4 of the 38 corpus cases are `known_bug: None`
  (bare integer/float/symbol literals) — a much lower "clean" fraction than
  `j-to-semantic-ir`'s 21-of-36, reflecting the actual, current state of
  the shared SIR23 JS backend rather than a shortfall in this frontend's
  own corpus design.

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
