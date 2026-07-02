# Changelog

All notable changes to the `coding-adventures-closure-pass-inline-variables` crate will be documented in this file.

## [0.10.0] - 2026-07-02

### Added — CLOC12.151: `ArrowFunctionExpression` traversal

Handle the new `Expression::ArrowFunctionExpression` variant by recursing into arrow bodies — both the block form (`x => { ... }`) and the concise/expression form (`x => expr`) — mirroring this pass's existing `FunctionExpression` handling. Part of the atomic `ArrowFunctionExpression` enum-variant rollout (javascript-ast 0.15.0); adding the variant makes every exhaustive `match` on `Expression` non-exhaustive, so all consumers gain their arm in one PR.

## [0.9.0] - 2026-07-01

### Added — CLOC12.149: propagate through `FunctionExpression` bodies

`count_uses_expr` and `propagate_in_expr` now recurse into a
`FunctionExpression` body (via the `_stmt` helpers), keeping the use
count and the substitution walk over the same positions. Over-counting
under param/self-name shadowing is conservative — it only declines an
inline, never performs a wrong one.

## [0.8.0] - 2026-07-01

### Added — upstream `InlineVariablesTest.java` conformance port (#88, CLOC12.146)

The **first** CLOC12 upstream-test port into this crate. New file
`tests/upstream/inline_variables_test.rs` (registered as the
`upstream_inline_variables` test target) reshapes upstream
`InlineVariablesTest.java` onto our surface, driving the **real** source →
`grammar_to_program` bridge → `InlineVariablesPass` → `emit` roundtrip, so each
case is `assert_eq!(propagate(src), expected)` on emitted JS.

- **13 active `#[test]`s pass on the first run** (no new propagation defect):
  single-use const-literal propagation, propagation into a larger expression,
  a short literal duplicated across multiple sites, boolean/null literals, the
  multi-use size budget (a long literal is declined at multiple sites but
  propagated at a single site), `let`/`var` never propagated, non-literal
  initializers declined, the shadowed-name guard, property names never
  replaced while computed member indices are, and two TDZ soundness cases
  (inert-prefix propagates; code-before-declaration declines).
- **3 `#[ignore = "blocked on gap-NNN"]` placeholders** pin the whole-program
  `InlineVariables` behaviors closurec does not do in this pass —
  gap-148 (single-assignment `let`/`var` inlining), gap-149 (identifier-alias
  initializers), gap-150 (removing the dead `const` husk, which
  `remove-unused-vars` owns). Each is pinned to `code/specs/CLOC12-gaps.md`.

This is a **test-only** change: no `src/` file is touched, so there is no
ripple into downstream consumers. Scaffolding files
`tests/upstream/{UPSTREAM_SHA,ATTRIBUTION.md}` were added per the CLOC12 port
convention.

## [0.7.0] - 2026-07-01

### Added — CV provenance for constant propagation (#89)

The pass now records every constant it propagates as a `propagated`
correlation-vector contribution carrying `{name, value, sites}` — the original
`const` name, a compact rendering of its literal value, and how many use sites
the literal replaced. Propagation *dissolves* the binding: its declaration
becomes unreferenced (remove-unused-vars deletes it) and the literal is copied
to each reader, so without this record the minified output has no trace that a
named constant ever stood there. These contributions let a `--correlation_vector`
consumer map an inlined literal back to the `const` it came from.

- Records emit in program (source) order, one per propagated constant, so the
  contribution list is deterministic run to run.
- `value` renders numbers/bigints from their raw text, strings quoted, and
  `true`/`false`/`null`/`undefined` literally.
- Attached at the program root — a coarse name→value/site-count *table*. Tagging
  each substituted literal's own CV id is a documented follow-up, mirroring the
  inline / rename passes.
- Emitted JS is byte-identical: contributions are pure metadata. Verified by the
  full closurec end-to-end suite.

`coding_adventures_correlation_vector` moves from a dev-dependency to a runtime
dependency (the pass now names `Contribution`), and `serde_json` is added for
the `json!` meta values. Three new unit tests cover a single-use propagation
(`sites: 1`), a multi-use propagation (`sites: 2`), and the no-propagation
(`let`, empty table) case.

## [0.6.1] - 2026-06-30

### Changed — test sync for closure-emitter boolean shorthand

`closure-emitter` 0.18.9 now minifies `true`/`false` to `!0`/`!1`. The
`propagates_boolean_and_null_literals` golden-output test was updated to
expect the new rendering (`const ON=!0;const NONE=null;f(!0,null);`). No
behavior change in this crate — the propagation logic is unchanged.

## [0.6.0] - 2026-06-20

### Added — CLOC23: variable inlining inside `for`-`of`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `ForOfStatement`, counting the `left` declaration as the loop-variable
binding — identical to the `for`-`in` handling.

## [0.5.0] - 2026-06-20

### Added — CLOC22: variable inlining inside `for`-`in`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `ForInStatement`. The for-in `left`, when a declaration, is counted as a
binding (the loop variable), mirroring the for-statement init handling.

## [0.4.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

The statement walks (`count_decl_names_stmt`, `count_uses_stmt`,
`propagate_in_stmt`) now cover `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. Added to keep the matches exhaustive over
the new AST variant.

## [0.3.0] - 2026-06-20

### Added — CLOC20: variable inlining inside `do`/`while`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `DoWhileStatement` (loop body and test), mirroring the existing `while`
handling so const-literal propagation reaches into do-while loops.

## [0.2.0] - 2026-06-20

### Added — CLOC19: variable inlining inside `try`/`catch`/`finally`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `TryStatement` (protected block, catch handler body, finalizer). The catch
`param` is counted as a declared binding in `count_decl_names_stmt` so a candidate
that shadows it is correctly excluded from propagation — preserving soundness when
a top-level name is also bound by a catch clause.

## [0.1.0] - 2026-06-17

### Added (CLOC13.H — constant propagation)

New crate per CLOC06's canonical pass set — Closure Compiler's `InlineVariables`
in miniature. `InlineVariablesPass::run` propagates a **top-level `const` bound
to a literal** to all of its use sites:

```js
const RATE = 2;
total = base * RATE;
// =>  const RATE = 2;   (now unreferenced — removed by remove-unused-vars)
//     total = base * 2;
```

- `InlinePass`-style metadata: `name = "inline-variables"`,
  `depends_on = ["constant-fold"]` (so a folded initializer `const X = 1 + 2`
  → `const X = 3` is a literal by the time we look), `iteration_policy =
  FixedPoint`, `cost = 3`.
- **Soundness** rests on three restrictions, plus the inline pass's
  self-contained shadow guard (the name must be declared exactly once in the
  whole program):
  - **`const` only** — a `let`/`var` can be reassigned between its declaration
    and a use, so its initializer is not a safe substitute. `const` cannot.
  - **literal values only** — a literal is immutable. `const X = y;` (an
    identifier whose value could later change) and `const X = o.p;` (a member
    read that could trigger a getter) are NOT propagated.
  - **temporal-dead-zone guard** — a `const` read before its declaration line
    runs throws `ReferenceError` (even from a function called early). We only
    propagate when every top-level item *before* the declaration is inert (a
    function declaration, or a variable declaration with only literal
    initializers), so nothing executes — and nothing can read the binding in
    its TDZ — before it initializes. Only single-declarator `const`s are taken.
- **Single-use** → always propagated (the whole `const` declaration becomes
  pure overhead once its one use is gone). **Multi-use** → propagated only when
  the literal's emitted form is short (`<= MAX_MULTIUSE_LITERAL_LEN`, 8 bytes),
  so duplicating it across the uses is outweighed by deleting the declaration.
- The pass only **propagates**; it leaves the emptied `const` declaration for
  `remove-unused-vars` to delete (mirrors how the inline pass leaves dead
  functions for treeshake). Property names (non-computed `.x` / object keys)
  and assignment targets are never substituted; computed `o[X]` is.
- Self-contained name-based analysis over the Phase-1 AST (same philosophy as
  the `inline` and `rename` passes); does not depend on `closure-scope-analyzer`.

### Tests
- 19 tests: metadata/pipeline-ordering contract + source → bridge →
  inline-variables → emit roundtrips covering single/multi-use propagation, the
  multi-use literal-size budget, and every rejection (let/var, non-literal
  value, shadowed name, property name, computed member).
