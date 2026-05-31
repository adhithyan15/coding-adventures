# CLOC12 — Upstream port gaps

> Status: tracking list. Seeded by CLOC12.01 with no entries.
> Populated as ported upstream tests get marked `#[ignore]`.

## Numbering

Every gap gets a `gap-NNN` ID, allocated in order. Once allocated, an
ID is never reused — even when the gap closes, the entry stays for
historical context with status `RESOLVED` and a link to the fix PR.

## Format

```
## gap-NNN — short title

- **Status:** OPEN | RESOLVED-in-#NNNN
- **Upstream test:** <ClassName>::<testName>
- **Ported file:**   tests/upstream/<file>.rs
- **Why it fails:**  one paragraph
- **What it needs:** one paragraph; usually points at a CLOC11.* slice or
                     a pass-body PR
```

## Entries

### gap-001 — typed AST lacks `undefined` / `NaN` / `Infinity` literal-equivalents

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testUndefinedComparison1`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream's `test("undefined == undefined", "true")` requires recognising the bare `undefined` identifier (and `NaN`, `Infinity`) as JS-spec literal-equivalents during the fold. The typed AST currently models them as plain `Identifier` nodes, so the fold pass sees identifiers and leaves them alone (the sound default).
- **What it needs:** Either a new `Expression::UndefinedLiteral` variant (mirroring `NullLiteral`) plus parser support that emits it for the `undefined` identifier, or a fold-pass extension that special-cases `Identifier { name: "undefined" }` in the no-shadowing case. Same treatment for `NaN`, `Infinity`. Likely a CLOC11.* slice once the parser bridge is in.

### gap-002 — constant-fold doesn't treat `void 0` as `undefined`

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testUndefinedComparison2`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** `void 0` is `UnaryExpression { operator: Void, argument: NumericLiteral(0) }` in our typed AST. Upstream folds it to `undefined`, which then participates in equality folds. We need both gap-001 first and a `void <literal>` → `undefined` fold rule.
- **What it needs:** A unary-`void`-of-literal fold rule plus the `undefined` literal from gap-001.

### gap-003 — cross-type `null == x` fold not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNullComparison1` (cross-type loose-equality lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Our `==` fold rule only fires when both literals are the same JS type (sound default per crate docs). The JS abstract-equality algorithm says `null == 0` is `false`, `null == "hi"` is `false`, etc., but folding that requires implementing the actual algorithm.
- **What it needs:** Implement the abstract-equality algorithm for the cases where both sides are compile-time constants. Mirror upstream's `PeepholeFoldConstants.tryFoldComparison`.

### gap-004 — abstract-equality and abstract-comparison folds across Number/String

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberStringComparison`, `PeepholeFoldConstantsTest::testStringNumberComparison`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream folds `1 < '2'` to `true` (string `'2'` coerced via `ToNumber`) and `1 == '2'` to `false` (loose equality, per ES spec §IsLooselyEqual). Our pass leaves mixed-type `==`/`<`/`>` alone.
- **What it needs:** Same shape as gap-003 — implement abstract-equality and abstract-relational-comparison for compile-time constants.

### gap-005 — `typeof` operator constant-fold not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testStringStringComparison` (lines using `typeof`)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Crate-level docs already flag `typeof` as deferred. `typeof <literal>` → corresponding string literal needs no runtime info; identity-comparison fold (`typeof x === typeof x` → `true`) is a separate optimization.
- **What it needs:** Implement `UnaryExpression { op: TypeOf, argument: literal }` → corresponding `StringLiteral` ("number" / "string" / "boolean" / "object" / "undefined" / "function").

### gap-007 — `NullLiteral OP NullLiteral` fold not implemented

- **Status:** RESOLVED in CLOC12.03 (PR pending)
- **Upstream test:** `PeepholeFoldConstantsTest::testNullComparison1` (`null OP null` self-relation lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution:** Added a `NullLiteral`/`NullLiteral` branch in `try_fold_binary_op` returning `Boolean(true)` for `==`/`===`/`<=`/`>=` and `Boolean(false)` for `!=`/`!==`/`<`/`>`. Relational ops follow ECMAScript §IsLessThan with `ToNumber(null) = 0`.

### gap-008 — cross-type strict equality fold (`Number === String → false`)

- **Status:** RESOLVED in CLOC12.03 (PR pending)
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberStringComparison` (`===`/`!==` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution:** Added a strict-equality-cross-type branch in `try_fold_binary_op` that fires after the same-type branches. Uses a new internal helper `js_literal_type` to tag each Phase 1 primitive literal with a discriminator (`"number"`/`"string"`/`"boolean"`/`"null"`); when both sides are tagged and tags differ, `===` → `false`, `!==` → `true`. Loose `==` is still untouched (gap-003/gap-004).

### gap-006 — unary plus / minus on identifiers, plus identifier-arithmetic shape

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberNumberComparison` (`+x > +y` `testSame` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream's `testSame("+x > +y")` asserts the pass leaves the expression alone (`x` is unknown). Our pass already does the right thing structurally; this gap is mostly the bookkeeping of porting the remaining `testSame` lines that use unary-plus on identifiers.
- **What it needs:** Trivial — extend the ported tests once `gap-005` lands so the batch reflects the full upstream method.

### gap-009 — `LabeledStatement` / `BreakStatement` not modelled in Phase 1 AST

- **Status:** OPEN
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testRemoveNoOpLabelledStatement`, `testRemoveUselessLabelWithFollowingBreak`
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** The Phase 1 typed AST in `javascript-ast` doesn't include `LabeledStatement` or `BreakStatement` variants. Upstream's `a: break a;` requires both.
- **What it needs:** Add Phase 1.x variants. `LabeledStatement { label: Identifier, body: Statement }` and `BreakStatement { label: Option<Identifier> }`. Then teach DCE that `a: break a;` collapses to empty. Probably a CLOC09 Phase 1.x amendment + a small DCE rule.

### gap-010 — block-flattening (single-child block collapse) not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testFoldBlock` (block-flattening lines)
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** Upstream collapses `{{foo();}}` to `foo();`, `{foo();{}}` to `foo();`, etc. We don't flatten nested blocks — DCE's responsibility is dead-after-terminator and empty-statement removal, not structural simplification.
- **What it needs:** Either extend DCE with a "block-with-single-statement → that statement" rule, or stand up a dedicated normaliser pass. Probably belongs in DCE for simplicity (~30 lines).

### gap-011 — `if`-with-constant-test collapse lives in `fold-control-flow`, not DCE

- **Status:** RESOLVED in CLOC12.06 — behaviour covered via CLOC12.05 (PR #4672)
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testIf`, `testHook` (`if`/ternary constant-test lines)
- **Ported file (original site):** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Where the behaviour actually lives now:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs` — `test_if_true_folds_to_consequent`, `test_if_false_folds_to_alternate`, `test_if_false_no_alternate_becomes_empty_statement`, `test_if_numeric_one_folds_to_consequent`, `test_if_numeric_zero_folds_to_alternate`, `test_if_nonempty_string_folds_to_consequent`, `test_if_empty_string_folds_to_alternate`, `test_if_null_folds_to_alternate` (the last one is the exact line upstream `testIf` has for `if(null){…}else{…}` → alternate).
- **Resolution note:** The DCE-side test stub (`test_if_with_constant_test_collapse`) was changed from `#[ignore]` to a tiny non-ignored marker that documents the cross-crate routing for future readers. Behavioural coverage stays in fold-control-flow where it belongs.

### gap-012 — `ConditionalExpression` cleanup lives in `constant-fold`, not DCE

- **Status:** ROUTING (not a missing feature)
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testHook` (`a ? b : c` cleanup)
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** Belongs in `closure-pass-constant-fold`. Some of these are already covered by the constant-fold inline tests; the rest will get ported when CLOC12.0N expands the `PeepholeFoldConstantsTest` coverage.
- **What it needs:** Re-port into `closure-pass-constant-fold/tests/upstream/`.

### gap-013 — useless-loop-body folding not in DCE

- **Status:** ROUTING (not a missing feature)
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testFoldUselessFor`, `testFoldUselessDo`, `testFoldEmptyDo`, `testMinimizeLoop_*`
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** `while(x()){x}` → `while(x());` and friends belong in `closure-pass-fold-control-flow`.
- **What it needs:** Re-port into `closure-pass-fold-control-flow/tests/upstream/` once that crate's port file lands.

### gap-014 — `SwitchStatement` not in Phase 1 AST

- **Status:** OPEN
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testOptimizeSwitch*` (a dozen tests)
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** No `SwitchStatement`, `SwitchCase`, `BreakStatement` in the Phase 1 typed AST.
- **What it needs:** Phase 1.x AST extension to model `switch (x) { case 1: ...; default: ...; }`, plus the switch-optimisation logic. Substantial — multiple PRs.

### gap-015 — `var` / `let` / `const` lifting and hoisting

- **Status:** ROUTING + missing feature
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testVarLifting`, `testLetConstLifting*`
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Why it fails:** Requires scope analysis to know what's reachable. Our DCE doesn't do scope analysis; that's the territory of `closure-pass-remove-unused-vars` and an eventual hoisting pass.
- **What it needs:** A dedicated hoisting / unused-vars cleanup pass. Likely lands as new content in `closure-pass-remove-unused-vars` rather than here.

### gap-016 — `if (x) S` → `x && S` rewrite not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldOneChildBlocks` (`if(x) foo()` → `x&&foo()` lines)
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Why it fails:** Upstream compacts a one-statement `if (test) consequent` (no alternate) into a `LogicalExpression { left: test, op: And, right: consequent }` wrapped in an `ExpressionStatement`. Our pass leaves `IfStatement` shapes alone when the test isn't a literal.
- **What it needs:** A rewrite rule in `fold_if_statement`: when `alternate.is_none()`, the consequent is exactly one `ExpressionStatement`, and the test isn't a literal, replace the `IfStatement` with an `ExpressionStatement` wrapping `test && consequent_expr`. Must preserve side-effect semantics — only fire when both sides are observably safe to reorder.

### gap-017 — `if (x) C else A` → `x ? C : A` rewrite not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldOneChildBlocks` (`if(x){foo()}else{bar()}` → `x?foo():bar()` lines)
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Why it fails:** Upstream rewrites an `IfStatement` with single-`ExpressionStatement` branches into an `ExpressionStatement` wrapping a `ConditionalExpression`. Our pass keeps the `IfStatement`.
- **What it needs:** A rewrite rule that recognises the `if (test) C else A` shape where both branches are single `ExpressionStatement`s and produces `ConditionalExpression { test, consequent, alternate }`.

### gap-018 — De Morgan / negation-swap rewrites not implemented

- **Status:** OPEN
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldConditionalDeMorgan`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Why it fails:** Upstream rewrites `if (!a) foo() else bar()` → `if (a) bar() else foo()` to push the negation out. We don't do this.
- **What it needs:** Detect a top-level `UnaryExpression { op: Not, .. }` test on an `IfStatement` (and on `ConditionalExpression`), strip the `Not`, and swap consequent / alternate.

### gap-019 — return-then-return through `if-else` → ternary return

- **Status:** OPEN
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldReturns`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Why it fails:** Upstream rewrites `if(x) return 1; else return 2;` to `return x ? 1 : 2;` — needs gap-017 (the ternary rewrite) plus a special case that recognises `ReturnStatement` branches.
- **What it needs:** Land gap-017 first, then add a `ReturnStatement`-aware shape recogniser on top.

### gap-020 — `ThrowStatement` not in Phase 1 AST

- **Status:** OPEN
- **Upstream test:** `PeepholeMinimizeConditionsTest::testMinimizeIfWithThrow`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Why it fails:** No `ThrowStatement` variant in our typed AST.
- **What it needs:** A Phase 1.x AST extension to model `throw expr;`. Then teach fold-control-flow that `if (x) foo() else throw 1` → `if (!x) throw 1; foo();` (the early-throw rearrangement).
