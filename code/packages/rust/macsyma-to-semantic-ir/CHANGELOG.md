# Changelog

## [0.1.1] - 2026-07-25

### Added

- **`tests/oracle.rs` — HML01 §7 oracle/golden testing, cross-checking
  `macsyma-runtime` (ground truth) against `macsyma_to_semantic_ir::
  compile_source` → `semantic_ir::Module` → `semantic_ir_to_javascript::
  compile` → a real `node` process.** Macsyma (and its thin-alias sibling
  Maxima) was one of the very first math-language frontends built in this
  rollout, predating the oracle-testing convention itself — every OTHER
  `-to-semantic-ir` crate now has one; this closes that gap. Modeled
  directly on `wolfram-to-semantic-ir/tests/oracle.rs` (the freshest,
  nearest SIR23/CAS sibling — both lower straight to the symbolic
  vocabulary with no host-language binding at lowering time). 34-case
  corpus: literal arithmetic precedence, right-associative `^`, unary
  minus binding looser than `^` (`-2^2 = -4`); exact-integer vs.
  genuine-rational division; the `x+0`/`x*1`/`x^1`/`x^0` free-symbol
  identity-law simplifications; every comparison in Macsyma's own spelling
  (`=`/`#`/`<`/`>`/`<=`/`>=` — `=` is equality, `:` is assignment) plus a
  3-term `and` chain (the n-ary fold), `or`, and `not` (keyword spellings,
  not `&&`/`||`/`!`); the `sin`/`cos`/`sqrt` exact-value elementary-
  function folds; `:` (`Assign`) binding and reading back across
  statements, including the self-referential-assign loop guard (`x : x`);
  `:=` (`Define`) and a call; `if`/`then`/`else` (both branches, plus the
  no-`else` `False`-fallback case); `[...]` list-literal elementwise
  evaluation; and two control-flow-as-data cases (`while`/`return`).
- **Genuinely new finding vs. `wolfram-to-semantic-ir`'s own oracle file:
  `f(x) := body` round-trips correctly end-to-end, unlike Wolfram's own
  documented gap.** Macsyma's `lower_assign` (`COLONEQ` branch) already
  splits the LHS into the exact 3-argument `Define(name, List(params),
  body)` record the shared JS `defineHandler`/`applyUserFunction` require
  — confirmed by `function_definition_and_call`'s `known_bug: None`.
- **Genuinely new finding: two control-flow-as-data cases
  (`while_head_is_unevaluated_symbolic_data_matching_verbatim`,
  `return_head_is_unevaluated_symbolic_data_matching_verbatim`) match
  verbatim between the native pretty-printer and the compiled side's
  generic `toDisplayString`, despite being non-atomic.** `While`/
  `ForEach`/`ForRange`/`Block`/`Return` have no registered handler on
  either side, so both rebuild an unevaluated `Apply` term via the
  identical "unknown head" fallback; neither `MacsymaDialect::
  function_name` nor the JS `toDisplayString` special-cases these
  synthetic head names, so their generic function-call rendering
  coincides — as long as no argument is itself a `List` (which the native
  walker DOES special-case, reopening the display gap one level down).
- **Maxima-coverage finding, documented in `tests/oracle.rs`'s own module
  doc rather than a separate corpus**: `maxima-to-semantic-ir` is a pure
  re-export of this crate's `compile`/`compile_source` (no shim), and
  `maxima-runtime::MaximaSession` is a thin façade over this crate's own
  sibling `macsyma-runtime::MacsymaSession`, formatting the exact same
  `output_text` (same `cas_pretty_printer::MacsymaDialect`) behind a
  `(%oN) ` REPL-echo prefix. A separate Maxima oracle corpus would
  therefore re-run the identical evaluator against the identical lowering
  and assert the identical strings — pure duplication, not additional
  coverage.
- Adds a dev-dependency on `coding-adventures-macsyma-runtime` (this
  frontend's own sibling native-runtime crate) for `tests/oracle.rs`'s
  ground truth only — the non-dev `[dependencies]` section still does not
  depend on it; lowering itself only ever needs the parse-tree shape.

### Fixed

- **`tests/test_validator.rs` asserted the JS backend rejects every module
  this frontend produces — stale since `semantic-ir-to-javascript` gained
  real SIR23 codegen.** All 7 tests in that file were failing on
  `origin/main` with "expected the JS backend to reject a module using
  SIR23 features" even though the backend now accepts them. Fixed by
  inverting the assertion (`assert_js_backend_accepts`, checking
  `check_module` returns no errors) and updating the module doc comment.
- Added `tests/e2e_node.rs`: actually compiles and runs representative
  Macsyma programs (arithmetic, a function definition+call, assignment,
  control-flow constructs, a multi-statement program) through `node`,
  proving the SIR23 codegen path is genuinely executable, not just
  statically accepted.

## [0.1.0] - 2026-07-13

### Added

- Initial `macsyma-to-semantic-ir` frontend crate (HML01 Stream B), the
  **second** to target SIR23 (symbolic-expression/pattern domain), sibling
  to `wolfram-to-semantic-ir`: `compile`/`compile_source` lowering
  `coding-adventures-macsyma-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`.
- Design: retargets `macsyma-compiler`'s own `Compiler::compile_node`
  rule-name dispatch (which already lowers this exact CST to
  `symbolic_ir::IRNode`) onto `semantic_ir::Expr`'s SIR23 vocabulary
  (`SymSymbol`/`SymApply`) instead — every construct (arithmetic,
  comparisons, lists, function application, `:`/`:=` assignment/
  definition, and every control-flow form: `if`/`elseif`/`else`, `while`,
  `for … in … do`, `for … thru/while/unless … do`, `block(…)`,
  `return(…)`) lowers to symbolic data, matching `symbolic_ir::IRNode`'s
  "everything is one apply-tree" design and Macsyma's own native runtime.
- Covers all 24 of `macsyma-parser`'s currently-implemented grammar
  productions.
- **Scope boundary, disclosed from day one**: Macsyma's grammar has no
  pattern-matching or rewrite-rule syntax at all (no `_`/blank, no `x_`
  named pattern, no `->`/`:>` rule arrow, no `/.`/`//.` replacement) — this
  crate therefore only ever constructs `Expr::SymSymbol`/`Expr::SymApply`
  (plus reused `IntLit`/`FloatLit`/`StrLit`), never `SymPatternBlank`/
  `SymPatternNamed`/`SymRule`/`SymReplaceAll`, and never observes
  `Feature::PatternMatching`. This also means `measure_depth_iterative`/
  `drop_iterative` only need a match arm for `Expr::SymApply`.
- `f(x, y) := body` lowers to a **3-argument** `Define(f, List(x, y),
  body)`, deliberately NOT Wolfram's 2-argument `Define(Apply(f, params),
  body)` shape — mirrors `macsyma-compiler::Compiler::compile_assign`'s own
  existing shape exactly. A bare `name := body` (no call-shaped LHS) falls
  back to `Define(name, List([]), body)`, also matching that compiler's
  existing behaviour.
- Recursion-depth hardening applied from day one, proactively carried over
  from `wolfram-to-semantic-ir`'s own four-round security-review history
  (see that crate's `CHANGELOG.md`) rather than discovered fresh:
  - `MAX_EXPR_DEPTH` (256), identical value and identical justification
    (both grammars' native-stack crash floors were independently measured
    to be nearly identical, since both share the same generic
    `GrammarParser` dispatch engine).
  - `check_chain_length` caps every flat operator-chain fold (`additive`/
    `multiplicative`/`logical_or`/`logical_and`) before any tree is built.
  - `check_postfix_chain_length` caps chained call application
    (`f(x)(y)(z)…`) — simpler than Wolfram's `add_chain_depth` cumulative
    budget, because Macsyma's `postfix` has only one suffix shape (a call)
    with no second axis (like Wolfram's `[[…]]` Part indexing) to multiply
    against; a plain per-chain group count is already an exact bound.
  - `check_if_chain_length` — a **new** guard for a DoS-risk class
    Wolfram's grammar has no equivalent of: Macsyma's `if_expr` folds a
    flat `{ elseif expr then expr }` repetition (one CST node, cheap to
    parse regardless of clause count) into a *nested* `If` `SymApply`
    chain, one level per clause. Verified with a dedicated boundary test
    (255 elseif clauses parses, 256 is rejected) and a DoS-scale regression
    test (3,000 elseif clauses rejected cleanly).
  - `check_apply_arg_count` caps `arglist`/`list` element counts as a
    defense-in-depth allocation-size backstop.
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement, closing the gap
    that per-construct guards (each scoped to one grammar node) don't
    compose across nested `(...)` boundaries — the exact class of bug
    `wolfram-to-semantic-ir`'s security review found and fixed the hard
    way; applied here proactively instead.
- `Feature::Floats` regression avoided proactively: `number_literal_expr`
  is an instance method (not a free function) specifically so every
  `FloatLit`-constructing branch can call `self.observed.add(Feature::
  Floats)` immediately — this is a confirmed, previously-shipped bug in
  both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir` (their
  float-literal code paths never added this feature, so any module with a
  float literal failed `semantic_ir::validate()`). A dedicated regression
  test (`float_literal_module_validates_and_declares_floats`) asserts a
  float-literal-only module both validates successfully and declares the
  feature.
- `compile_source` needs no worker-thread stack enlargement, unlike
  `wolfram-to-semantic-ir::compile_source`: `macsyma-parser`'s own
  `MAX_RULE_DEPTH` (200) is already documented safe on a bare default
  (~2 MiB) stack with comfortable margin, so this crate mirrors
  `matlab-to-semantic-ir`'s and `apl-to-semantic-ir`'s simpler
  `compile_source` shape instead.
- 48 tests: 39 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (flat-chain, chained call
  application, if/elseif chain, deeply nested parens, and exact-boundary
  cases) and the `Feature::Floats` regression, 7 validator/capability-
  rejection tests, 1 doctest.
- Marks `macsyma-to-semantic-ir` done in `HML01-math-to-semantic-ir.md`'s
  Stream B rollout.

### Known limitation (disclosed, not a bug)

No module this crate produces currently executes end-to-end through any
backend: under the "everything is symbolic data" design inherited from
`macsyma-compiler`, even bare literal arithmetic emits at least one SIR23
node, and `semantic-ir-to-javascript` does not implement SIR23 codegen yet
(it depends on `sir-runtime-symbolic`, a separate, not-yet-shipped runtime
library — HML01 Stream B rollout item 6). This crate's tests therefore
verify structural correctness and the capability-rejection path only; there
is no e2e `node`-execution test, unlike `matlab-to-semantic-ir`'s
purely-literal case.
