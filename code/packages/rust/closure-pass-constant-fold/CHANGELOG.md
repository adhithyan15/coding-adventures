# Changelog

All notable changes to the `coding-adventures-closure-pass-constant-fold` crate will be documented in this file.

## [0.2.0] - 2026-05-24

### Added — real `Pass::run` body (first non-identity optimization)

Now that `javascript-ast` Phase 1 (CLOC09) is in main, `constant-fold` becomes the first pass that does real work. The `Pass::run` body is a recursive bottom-up walker over `Program → ProgramItem → Statement → Expression` that collapses every compile-time-evaluable subexpression:

**Arithmetic on `NumericLiteral` pairs** — `+`, `-`, `*`, `/`, `%`, `**`. `2 + 3 → 5`, `5 ** 8 → 390625`, etc.

**String concatenation and mixed-type coercion for `+`** — `"foo" + "bar" → "foobar"`, `"x" + 1 → "x1"`, `2 + "x" → "2x"`. Per ECMAScript, if either operand is a string then `+` is concatenation. Numbers stringify via the JS `String(n)` convention (`42` not `42.0`).

**Comparison** — `==`, `!=`, `===`, `!==`, `<`, `<=`, `>`, `>=` on matching literal types (number/number, string/string, boolean/boolean). Mixed-type loose equality (`1 == "1"`) is **not** folded — sound default until we have an explicit toggle.

**Logical short-circuit** — `false && X → false`, `true && X → X`, `true || X → true`, `false || X → X`, `null ?? X → X`, `0 ?? X → 0`. Folds when the LEFT side is a literal; right side may be any expression (including identifiers/calls that would have evaluation side effects — we elide them because the JS short-circuit semantics say they wouldn't have run).

**Unary** — `!` on any literal (numeric → boolean via truthiness; string → boolean via length; null → true; boolean → flipped), `-` on numeric, `+` on numeric / boolean / null / parseable-numeric-string.

**Conditional (ternary)** — `true ? a : b → a`, `0 ? a : b → b`. Test must be a literal we can judge for truthiness.

**Recursion** — the walker descends through every Phase 1 node type: `Statement` (including `IfStatement.test`, `WhileStatement.test`, `ForStatement.{init,test,update,body}`, `ReturnStatement.argument`, `BlockStatement.body`), `Declaration` (including `VariableDeclarator.init` and `FunctionDeclaration.body`), and every `Expression` (`AssignmentExpression.right`, `CallExpression.{callee,arguments}`, `MemberExpression.{object,property}`, `ArrayExpression.elements`, `ObjectExpression.properties`). So `1 + (2 * 3) → 1 + 6 → 7` happens in a single bottom-up pass.

### CV tracing — both modes work

Per the CLOC09 amendment:
- **Traced input** (`cv: Some(parent)`) → folded replacement gets a new id via `CVLog::derive(parent, None)`, and a `Contribution { source: "constant-fold", tag: "folded", meta: {before, after, parent_cv, new_cv} }` is appended.
- **Untraced input** (`cv: None`) → folded replacement also has `cv: None`, **no** contribution is emitted. The `changed: true` flag is still set so the pipeline knows something happened.

Both modes verified by separate tests (`fold_in_untraced_mode_skips_cv_and_contributions`).

### Skipped (intentionally) for v0.2.0 — queued for v0.3.0+
- `typeof`, `void` — need an undefined-literal node (Phase 1 doesn't have one).
- `delete` — has observable side effects.
- Bitwise (`&`, `|`, `^`, `<<`, `>>`, `>>>`) — needs int32 coercion semantics; queued for v0.3.0 once test fixtures drive demand.
- Mixed-type loose equality — sound default; opt-in toggle planned.
- `AssignmentExpression`, `CallExpression`, `MemberExpression`, etc. — recursed-through but not collapsed (require runtime knowledge / have side effects).

### Tests
27 tests covering: pass metadata (unchanged from v0.1.0), empty-program identity (still produces no contributions), each arithmetic operator, each comparison operator, string concatenation in both directions, every unary operator we support, every logical operator with both left-wins and right-wins paths, conditional with both truthy and falsy tests, **nested folding in a single bottom-up pass** (`1 + (2 * 3) → 7` with 2 contributions emitted), unfoldable expressions pass through unchanged with `changed: false`, mixed-type loose equality is preserved, **untraced mode** (cv: None) produces no contributions but still folds, recursion through `VariableDeclarator.init` and `IfStatement.test/consequent/alternate`, pipeline integration.

### Notes
- The implementation is split into module-internal helpers (`fold_program`, `fold_statement`, `fold_expression`, `fold_binary`, `fold_logical`, `fold_unary`, `fold_conditional`) plus a `FoldState` struct that threads CV log + accumulators through the walk.
- `try_fold_binary_op` is a pure function (no I/O, no CV) that returns `Option<FoldedLiteral>` — separated from the IO-touching wrapper so the fold *semantics* are testable independently of CV bookkeeping in future tests.
- `format_js_number(n)` renders numbers the way JS's `String(x)` does (`42` not `42.0`, `0.5` not `.5`, `NaN`/`Infinity` literal-cased) so `"x" + 1 === "x1"` not `"x1.0"`.
- The `lit_label` / `literal_label` / `op_label` family produces human-readable strings for the `before` / `after` fields of the emitted `Contribution.meta` — useful for debugging via the CV log.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06 — first concrete optimization pass plugged into the `closure-pass-pipeline` harness.
- `ConstantFoldPass` zero-sized type implementing `Pass`:
  - `name = "constant-fold"`
  - `iteration_policy = IterationPolicy::FixedPoint` (folds expose further folds; full multi-iteration loop arrives when the pipeline grows past v0.1.0)
  - `cost = 2` pass-units (tree walk + small constant work per visit)
  - `depends_on()` / `invalidates()` empty in v1
- `ConstantFoldPass::new()` zero-arg constructor for ergonomic `PassPipeline::add(Box::new(ConstantFoldPass::new()))` registration.
- `Pass::run` is **identity** in v1: `javascript-ast` ships only `Program` / `SourceType` today (CLOC02 Phase 1), so there's nothing to fold. The pass clones the input `Program` unchanged, returns `changed = false`, `nodes_touched = 1`, no contributions (per CLOC03 §"When a pass keeps a node unchanged").
- 8 tests covering: `name()` value, `iteration_policy` is FixedPoint, `cost` is 2, `depends_on`/`invalidates` empty, run on empty Program is identity (program unchanged, no contributions, stats correct), full `PassPipeline` integration as solo pass (verifies FixedPoint note diagnostic flows through), pipeline integration alongside an unrelated upstream pass (registration order preserved), pass is `Default` + `Clone`.

### Notes
- Dependencies: `coding-adventures-closure-pass-pipeline` (Pass trait + types), `coding-adventures-javascript-ast` (Program), `coding-adventures-type-sidecar` (future type-aware fold safety), `coding_adventures_correlation_vector` (Contribution plumbing), `serde_json` (meta JSON values). Dev-dep: `coding-adventures-javascript-tokens` for `EsVersion` in tests.
- v1 is scaffolding. Real folding (number/string/boolean/typeof/negation/comparison/conditional) lands once `javascript-ast` grows `Statement` / `Expression` variants — at that point this file becomes a real pass without any API churn upstream.
