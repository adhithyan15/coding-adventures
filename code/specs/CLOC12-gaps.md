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

- **Status:** PARTIALLY RESOLVED in CLOC12.16 — `UndefinedLiteral` is now in the AST. `NaN` and `Infinity` remain open (tracked under a follow-up since they're `Identifier { name: "NaN"/"Infinity" }` resolutions, not new literal nodes).
- **Upstream test:** `PeepholeFoldConstantsTest::testUndefinedComparison1`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution note:** CLOC12.16 adds `UndefinedLiteral { cv: Option<CvId> }` to `javascript-ast`. The closure-emitter writes it as `void 0` (shadow-safe — `undefined` is a writable identifier in non-strict mode, but `void <expr>` always produces the genuine undefined value). The closure-pass-constant-fold gained three new arms: leaf passthrough, `js_literal_type` → `"undefined"`, and `typeof <UndefinedLiteral>` → `"undefined"` (closes the last hole in CLOC12.09's typeof table). The cross-type strict-equality fold also automatically picks up undefined vs other types because `js_literal_type` produces a distinct tag for it.

### gap-002 — constant-fold doesn't treat `void 0` as `undefined`

- **Status:** RESOLVED in CLOC12.20 — `UnaryOperator::Void` over a primitive literal now folds to `UndefinedLiteral`. `closure-pass-constant-fold 0.8.0` adds the `Void` arm in `fold_unary` and the `FoldedLiteral::Undefined` variant; `test_undefined_comparison_2` is un-ignored.
- **Upstream test:** `PeepholeFoldConstantsTest::testUndefinedComparison2`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it failed:** `void 0` is `UnaryExpression { operator: Void, argument: NumericLiteral(0) }` in our typed AST. Upstream folds it to `undefined`, which then participates in equality folds. We needed both gap-001 first (UndefinedLiteral variant, closed in CLOC12.16) and a `void <literal>` → `undefined` fold rule (this PR).
- **What it needed:** A unary-`void`-of-literal fold rule plus the `undefined` literal from gap-001. Both shipped.

### gap-003 — cross-type `null == x` fold not implemented

- **Status:** RESOLVED in CLOC12.21 — `try_fold_binary_op` now implements the `null`-side branch of ECMAScript §IsLooselyEqual for compile-time-known partner literals. `null == X` (or `X == null`) folds to `true` iff `X` is `null` (the existing gap-007 branch, untouched) or `undefined`; every other primitive-literal partner (`number`, `string`, `boolean`, `bigint`) folds to `false`. `!=` is the boolean negation. Identifier-on-other-side bails out — the runtime value could itself be null/undefined, and folding to a concrete boolean would be unsound. `test_null_comparison_1_loose_against_other_types` is un-ignored. Inline tests cover both directions (left/right swap of null), the `!=` complement, the `null == undefined → true` special case (both directions), the unsoundness guard against identifiers, and a regression check that gap-008's strict-equality cross-type branch still fires.

### gap-004 — abstract-equality and abstract-comparison folds across Number/String

- **Status:** RESOLVED in CLOC12.22 — `try_fold_binary_op` now coerces a String operand against a Number operand via a conservative subset of §StringToNumber (`js_string_to_number_strict`) and evaluates the resulting Number-vs-Number comparison for `==` / `!=` / `<` / `<=` / `>` / `>=`. Order is preserved: `'2' < 1` still folds to `false` (NOT swapped to `1 < 2`). Operates on both directions (Number-on-left and String-on-left). Strict `===` / `!==` is unaffected — gap-008's cross-type branch already handles those and is statically unreachable from this branch's operator guard. The string→number helper recognises the empty/whitespace string, decimal literals (with optional sign, `.`, and `[eE][+-]?` exponent), and the explicit `Infinity` / `+Infinity` / `-Infinity` forms; it deliberately *bails* on hex/binary/octal prefixes, lone signs/dots, non-ASCII whitespace, and unrecognised text — those are sound follow-ups that can be added without re-deriving the rules. `test_number_string_comparison_literal_lines` is un-ignored.
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberStringComparison`, `PeepholeFoldConstantsTest::testStringNumberComparison`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`

### gap-005 — `typeof` operator constant-fold not implemented

- **Status:** RESOLVED in CLOC12.09 (literal-typeof cases) — residual identity-fold tracked as gap-029
- **Upstream test:** `PeepholeFoldConstantsTest::testStringStringComparison` (lines using `typeof`)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution:** Added a `UnaryOperator::TypeOf` branch to `fold_unary` that pattern-matches the argument against the four Phase 1 primitive literals and returns the corresponding string: `NumericLiteral → "number"`, `StringLiteral → "string"`, `BooleanLiteral → "boolean"`, `NullLiteral → "object"` (the JS quirk). The remaining cases (`undefined`, BigInt, function expression, identifier) stay deferred per their respective gaps (gap-001, gap-021, Phase 1.x AST, runtime-unknown). The identity-comparison fold (`typeof x === typeof x` → `true`) is conceptually different — see gap-029.

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

- **Status:** RESOLVED in CLOC12.23 — pure test-only port; no production code changed. The pass already declines to fold `+<identifier>` / `-<identifier>` in `fold_unary` (runtime value is unknown), and the surrounding `try_fold_binary_op` declines whenever either side isn't a recognised literal. The new `test_same_unary_on_identifier_in_comparison` test in `peephole_fold_constants_test.rs` pins the upstream `testSame("+x > +y")` / `testSame("+x == +y")` lines plus several adjacent shapes: `+x === +y`, `-x < -y` (Negate variant), asymmetric `0 < +x`, and `+x == +x` (same identifier on both sides — must NOT fold because `x` could be NaN at runtime).
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberNumberComparison` (`+x > +y` `testSame` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`

### gap-009 — `LabeledStatement` / `BreakStatement` not modelled in Phase 1 AST

- **Status:** RESOLVED in CLOC12.13 (AST modelled; `testRemoveNoOpLabelledStatement` now exercises real nodes). Residual: the actual *collapse* of `a: break a;` to empty is its own follow-up gap (tracked in-test).
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testRemoveNoOpLabelledStatement`, `testRemoveUselessLabelWithFollowingBreak`
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Resolution note:** `BreakStatement` was already modelled in the original Phase 1 implementation; `LabeledStatement` was the missing piece. CLOC12.13 adds `LabeledStatement { label: Identifier, body: Box<Statement>, cv }` to `javascript-ast`, wires it through constant-fold / fold-control-flow / DCE (passthrough — recurse into body, label preserved), and teaches `closure-emitter` to print `label:body`. The test stub was un-ignored and rewritten to build the `a: break a;` AST by hand and assert the pipeline preserves it verbatim (which is the *current* behaviour). When the collapse optimisation lands the assertion flips from `assert_dce_same` → `assert_dce_yields(..., vec![])`.

### gap-010 — block-flattening (single-child block collapse) not implemented

- **Status:** RESOLVED in CLOC12.19 (PR pending).
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testFoldBlock` (block-flattening lines)
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Resolution note:** Added a flatten step at the top of `dce_block_statement` (after the recurse pass, before dead-after-terminator and EmptyStatement drops). For each direct-child statement, if it's a `BlockStatement` AND a new `block_is_scope_safe_to_flatten` helper returns true (i.e., contains no `let`/`const`/`class`/`function` declarations), splice its body into the enclosing block. `var` is fine because it's function-scoped. Cascades cleanly with the existing dead-code drops: `{x;{return;y;};z;}` → `{x;return;}` in one pass. `test_fold_block_flattening` un-ignored with four assertions; existing `recurses_into_nested_blocks` updated to reflect the new behaviour.

### gap-011 — `if`-with-constant-test collapse lives in `fold-control-flow`, not DCE

- **Status:** RESOLVED in CLOC12.06 — behaviour covered via CLOC12.05 (PR #4672)
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testIf`, `testHook` (`if`/ternary constant-test lines)
- **Ported file (original site):** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Where the behaviour actually lives now:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs` — `test_if_true_folds_to_consequent`, `test_if_false_folds_to_alternate`, `test_if_false_no_alternate_becomes_empty_statement`, `test_if_numeric_one_folds_to_consequent`, `test_if_numeric_zero_folds_to_alternate`, `test_if_nonempty_string_folds_to_consequent`, `test_if_empty_string_folds_to_alternate`, `test_if_null_folds_to_alternate` (the last one is the exact line upstream `testIf` has for `if(null){…}else{…}` → alternate).
- **Resolution note:** The DCE-side test stub (`test_if_with_constant_test_collapse`) was changed from `#[ignore]` to a tiny non-ignored marker that documents the cross-crate routing for future readers. Behavioural coverage stays in fold-control-flow where it belongs.

### gap-012 — `ConditionalExpression` cleanup lives in `constant-fold`, not DCE

- **Status:** RESOLVED in CLOC12.27 — the upstream `testHook` test was re-routed to `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs::test_hook_ternary_cleanup_sequence_dependent` (an `#[ignore]`-d stub), and the DCE port file's `test_hook_cleanup` was reannotated to point at the new home. The literal-test ternary cases (`true ? c : a` → `c`, `false ? c : a` → `a`) are already covered by `fold_conditional` + `literal_truthy` in the constant-fold crate's inline tests. The SequenceExpression-dependent rewrites (`a ? X : X` → `(a, X)`) are *not* a fold-rule gap but a Phase 1.x AST gap — they require `javascript-ast` to grow a `SequenceExpression` variant before they can be represented, let alone folded. Tracked separately under "needs SequenceExpression" — *not* under any CLOC12 gap-NNN entry, because the missing piece is a primary AST node rather than a constant-fold rule.
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testHook` (`a ? b : c` cleanup)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs` (re-routed in CLOC12.27); the original DCE port file points to the new home via a doc comment.

### gap-013 — useless-loop-body folding not in DCE

- **Status:** RESOLVED in CLOC12.28 — the upstream `testFoldUselessFor` / `testFoldUselessDo` / `testFoldEmptyDo` / `testMinimizeLoop_*` tests were re-routed to `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs::test_fold_useless_loop_body_routing` (an `#[ignore]`-d stub) and the DCE port file's `test_fold_useless_loop_body` was reannotated to point at the new home. The literal-test loop-collapse cases (`while(false) { ... }` → `;`) are already covered by `fold_while_statement` + `literal_truthy` in the fold-control-flow crate's inline tests. The body-pure-no-effects rewrites (`while(x){pure(...)}` → `while(x);`) are *not* a fold-rule gap but an effect-analysis gap — they require a pass that can prove "this statement has no observable side effects" before the body can be dropped. Tracked separately as a future "effect analysis" gap, *not* under any CLOC12 gap-NNN entry, because the missing piece is a primary analysis rather than a fold rule.
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testFoldUselessFor`, `testFoldUselessDo`, `testFoldEmptyDo`, `testMinimizeLoop_*`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs` (re-routed in CLOC12.28); the original DCE port file points to the new home via a doc comment.

### gap-014 — `SwitchStatement` not in Phase 1 AST

- **Status:** **RESOLVED**. AST + emitter in CLOC12.33; empty-switch elimination in CLOC12.34 (closure-pass-dce 0.6.0); drop-after-break in CLOC12.35 (0.7.0); constant-discriminant collapse in CLOC12.36 (0.8.0 — strict-equality match against pure-leaf-literal discriminant + tests, trailing-break strip, fallback to default, conservative bail on fall-through or NaN). The fall-through-aware multi-case concatenation variant is the only remaining elaboration and is tracked as a future polish rather than a gap.
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testOptimizeSwitch*` (a dozen tests)
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`
- **Resolution note:** CLOC12.33 adds `SwitchStatement { cv, discriminant, cases }` and `SwitchCase { cv, test: Option<Expression>, consequent: Vec<Statement> }` to `javascript-ast`, plus the `TaggedStatement::SwitchStatement` arm and `Statement::switch_statement` convenience constructor. The closure-emitter learns `emit_switch` / `emit_switch_case` covering both compact and pretty modes (compact `switch(x){case 1:y;default:z;}`, pretty indented). DCE / fold-control-flow / constant-fold / scope-analyzer each gained a passthrough match arm that recurses into the discriminant, each case's test, and each consequent statement — no peephole rules yet. The structural prerequisite is in place; the per-test arms (empty-switch removal, etc.) land in CLOC12.34+ as separate slices. `BreakStatement` was already in the AST (CLOC12.13).

### gap-015 — `var` / `let` / `const` lifting and hoisting

- **Status:** RESOLVED in CLOC12.37 — var-hoisting landed in `closure-pass-fold-control-flow 0.8.0` as a post-step inside the `Declaration::FunctionDeclaration` arm. After the body folds, `var x = expr;` declarations nested in blocks / ifs / whiles / fors / labels / switch-cases lift to a single prepended `var x, y, z;` at the function-body top, with `x = expr;` assignment-statements remaining at the original sites. `let` and `const` (block-scoped per spec) stay put — the spec-correct behaviour, the gap title was misleading. The upstream "let/const lifting" optimisations are about removing redundant bindings; when those come up they'll route through `closure-pass-remove-unused-vars` (CLOC13.E), not here.
- **Upstream test:** `PeepholeRemoveDeadCodeTest::testVarLifting`, `testLetConstLifting*`
- **Ported file:** `closure-pass-dce/tests/upstream/peephole_remove_dead_code_test.rs`

### gap-016 — `if (x) S` → `x && S` rewrite not implemented

- **Status:** RESOLVED in CLOC12.24 — `fold_if_statement` now has a third (after the literal-truthy/falsy and gap-017 if-else→ternary branches): when `alternate.is_none()` AND the consequent reduces to a single `ExpressionStatement` (directly or via `single_expr_stmt`'s BlockStatement unwrap), the IfStatement is rewritten to `ExpressionStatement { LogicalExpression { left: test, op: And, right: consequent_expr } }`. Side-effect semantics are preserved: `&&` evaluates `test` first and the right operand only when `test` is truthy — exactly matching `if (test) S`'s observable behaviour. The rule does NOT fire when an alternate is present (preventing silent loss of the else branch) and does NOT fire when the consequent is multi-statement (no single right-hand expression to lower into). `test_fold_one_child_blocks_if_to_logical_and` is un-ignored. Two pre-existing inline tests were updated to reflect the new behaviour: `if_non_literal_test_with_no_alternate_passes_through` → `..._with_multi_statement_consequent_passes_through` (uses a 2-statement block that still can't fold), and `if_with_unresolved_comparison_doesnt_fold_alone` → `..._folds_via_gap016` (now expects the `(1<2) && A` shape and asserts the inner `1 < 2` survived as a BinaryExpression — confirming fold-control-flow still doesn't fold binary comparisons).
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldOneChildBlocks` (`if(x) foo()` → `x&&foo()` lines)
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`

### gap-017 — `if (x) C else A` → `x ? C : A` rewrite not implemented

- **Status:** RESOLVED in CLOC12.18 (PR pending).
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldOneChildBlocks` (`if(x){foo()}else{bar()}` → `x?foo():bar()` lines)
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Resolution note:** Added a rewrite arm in `fold_if_statement` that fires when the test isn't a literal AND both branches reduce to a single `ExpressionStatement` (recursively unwrapping single-statement `BlockStatement` layers via a `single_expr_stmt` helper). Emits an `ExpressionStatement` wrapping a `ConditionalExpression`. Side-effect-safe because ternary preserves the same evaluation order as if-else (test first, then exactly one branch). `test_fold_one_child_blocks_if_else_to_ternary` un-ignored.

### gap-018 — De Morgan / negation-swap rewrites not implemented

- **Status:** RESOLVED in CLOC12.25 — both `fold_if_statement` and `fold_conditional` now strip a top-level `!` from the test and swap consequent / alternate. The IfStatement case is gated on `alternate.is_some()` (when no alternate is present, gap-016's `!x && S` form is the better rewrite); the ConditionalExpression case has no guard because ternaries always have both arms. The unary's argument is moved (not cloned) into the new test position, so no second runtime evaluation of `<inner>` is introduced. Position: runs BEFORE `literal_truthy` resolution so the swapped tests can pick up literal folds in the same iteration. `test_fold_conditional_de_morgan` is un-ignored — input `if (!a) { foo() } else { bar() }` now folds through gap-018 swap → gap-017 ternary to `a ? bar() : foo();`.
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldConditionalDeMorgan`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`

### gap-019 — return-then-return through `if-else` → ternary return

- **Status:** RESOLVED in CLOC12.26 — `fold_if_statement` now has a fourth rewriting branch (after gap-018 swap, literal_truthy, gap-017 ternary, and now gap-019 — gap-016 stays after, gated on no alternate). When both branches reduce to a single `ReturnStatement` whose `argument` is `Some` (recursing through single-statement `BlockStatement` layers via the new `single_return_with_arg` helper), the IfStatement is replaced with `return test ? E1 : E2;`. The conservative `Some`-on-both-sides guard skips the `return;` (no argument) case — synthesising an `undefined` expression for it requires `UndefinedLiteral` plumbing in this pass and is tracked separately. `test_fold_returns_into_ternary` un-ignored. Composes with the gap-018 De Morgan swap so `if (!x) return E1; else return E2;` → (gap-018) `if (x) return E2; else return E1;` → (gap-019) `return x ? E2 : E1;`.
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldReturns`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`

### gap-020 — `ThrowStatement` not in Phase 1 AST

- **Status:** RESOLVED in CLOC12.14 (AST modelled). Residual: the actual *rewriting* of `if (x) foo(); else throw e;` → `if (!x) throw e; foo();` is a follow-up; modelling the node is the structural prerequisite.
- **Upstream test:** `PeepholeMinimizeConditionsTest::testMinimizeIfWithThrow`
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Resolution note:** CLOC12.14 adds `ThrowStatement { cv: Option<CvId>, argument: Expression }` to `javascript-ast` (argument non-optional per ECMAScript §13.14 — `throw;` is a SyntaxError). Wires it through `closure-emitter` (`throw expr;` with required whitespace between keyword and expression) and through all three rewriting passes (constant-fold, fold-control-flow, DCE — each folds the argument expression and preserves throw semantics; dead-after-throw collapse and the if-else early-throw rearrangement are separate follow-ups).

### gap-021 — `BigIntLiteral` not in Phase 1 AST

- **Status:** RESOLVED in CLOC12.15 (AST modelled; emitter writes the literal verbatim; `typeof <BigIntLiteral>` folds to `"bigint"` in constant-fold).
- **Upstream test:** `CodePrinterTest::testBigInt`
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution note:** CLOC12.15 adds `BigIntLiteral { cv: Option<CvId>, value: String, raw: String }` to `javascript-ast`. Per ESTree's JSON-safety convention `value` is the decimal expansion as a string (bigints can exceed `f64` range); `raw` keeps the source representation (including the trailing `n` and the source radix). The emitter writes `raw` verbatim — preserving hex/octal/binary radixes — because there is no shorter equivalent form for bigints (no exponential bigint syntax exists in JS). Negative bigints (`-5n`) are a `UnaryExpression` over a `BigIntLiteral`, never part of the literal itself. Bigint arithmetic folding (`1n + 2n → 3n`) is **not** implemented — would require a bigint runtime in the constant-fold pass; tracked as a follow-up gap. The `typeof <BigIntLiteral>` → `"bigint"` fold IS implemented since it requires no arithmetic.

### gap-022 — Array/object trailing-comma policy not modelled

- **Status:** RESOLVED in CLOC12.32. **What the gap actually needed**: a focused port file demonstrating that our emitter never emits a trailing comma before `]` or `}` in either compact or pretty mode. Upstream's `assertPrettyPrint("var x = [1,];", "var x = [1];\n")` family relies on the parse step stripping the trailing comma (it's purely syntactic in ES2017, NOT an elision) and the emitter simply never writing one. Our `ArrayExpression.elements: Vec<Option<Expression>>` doesn't preserve trailing-comma input — a parsed `[1,]` collapses to `[Some(1)]`, identical to `[1]`. So the `trailing_comma: bool` AST flag the original gap entry asked for is unnecessary; the output-side invariant is what matters, and the emitter already obeys it. 16 hand-built ports in `closure-emitter/tests/upstream/code_printer_trailing_comma_test.rs` pin the array/object/nested cases in both compact and pretty modes, plus the "elision is NOT a trailing comma" edge case.
- **Upstream test:** `CodePrinterTest::testTrailingCommaInArrayAndObjectWithPrettyPrint` and ~6 sibling tests
- **Ported file:** `closure-emitter/tests/upstream/code_printer_trailing_comma_test.rs` (new home); placeholder in `closure-emitter/tests/upstream/code_printer_test.rs` re-annotated to point at the new file.

### gap-023 — `VariableDeclaration` round-trip ports deferred

- **Status:** RESOLVED in CLOC12.30 — landed the focused declarations port file at `closure-emitter/tests/upstream/code_printer_declarations_test.rs` with 14 hand-built ports covering var/let/const, bare vs. with-init, single vs. multi-declarator, and the canonical `var x = [];` / `var x = [1];` / `var x = [1, 2, 3];` shapes (which also covers upstream's `testNoTrailingCommaInEmptyArrayLiteral` from the original code-printer port file). The original stub in `code_printer_test.rs` is reannotated to point at the new home. Verbosity is contained to one file behind two small helpers (`var_decl_single` / `var_decl_multi`). One pin documents existing emitter behaviour where it diverges from upstream's compact form (`a + b` vs `a+b` around binary `+`) — that style-policy delta is its own follow-up and not part of this gap.
- **Upstream test:** Most `assertPrintSame("var x = …")` lines in `CodePrinterTest`
- **Ported file:** `closure-emitter/tests/upstream/code_printer_declarations_test.rs`

### gap-024 — `ExpressionStatement` paren-wrapping diverges from upstream

- **Status:** RESOLVED in CLOC12.10
- **Upstream test:** Every upstream `assertPrintSame("foo();")` / `assertPrint("a+b", "a+b")` line
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** Added a precedence ladder (`PREC_PRIMARY`, `PREC_UNARY`, `PREC_CONDITIONAL`, `PREC_ASSIGNMENT`, plus per-operator `binary_prec` / `logical_prec` / `expr_prec`) and a `emit_expression_inner(e, parent_prec)` helper that wraps in parens only when the child's own precedence is strictly less than the parent context's. Statement position uses `parent_prec = 0`, so primary / call / member / binary / etc. emit without outer parens. The leading-token disambiguation wrap stays in place for `ObjectExpression` at statement position. The two `_is_current_behaviour` ports got their assertions flipped to upstream-byte-identical forms (`"2 + 3;"`, `"\"a\" + \"b\";"`). Three inline tests also flipped.

### gap-025 — Numeric formatting (shortest-form / exponential) not implemented

- **Status:** RESOLVED in CLOC12.12
- **Upstream test:** `CodePrinterTest` lines like `assertPrint("1000000000", "1E9")`
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** `format_js_number` now computes both decimal and exponential forms for finite non-zero numbers and returns the shorter (ties → decimal). `format_exponential_uppercase` wraps Rust's `{:e}` formatter and uppercases the `e`. Examples: `1000000000` → `1E9`, `5000000` → `5E6`, `1.5e-10` → `1.5E-10`. Small integers and decimals stay decimal. NaN/Infinity unchanged. **CLOC12.138:** the `test_number_formatting_shortest_form` placeholder is now an **active** conformance test (`1E9`, `1E6`, `1E21`, `100`→decimal tie, `0.5`, `-0`) — the follow-up re-port is done. (Residual finer optimisation not yet done: Closure drops the leading zero on `0.5` → `.5`; ours keeps `0.5`. Filed as gap-133 below.)

### gap-026 — String quote-choice optimisation not implemented

- **Status:** RESOLVED in CLOC12.11
- **Upstream test:** `CodePrinterTest` quote-choice lines
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** Added `choose_quote_and_escape(value)` + `escape_str_sq` helpers in `closure-emitter/src/lib.rs`. `emit_string` now picks single-quote when value contains strictly more `"` than `'` (each saved `\"` is one fewer escape); double-quote otherwise (canonical, ties picked toward double). `ascii_only` mode still always uses double — that's upstream's own invariant. Six new inline tests cover all branches. **CLOC12.138:** the `test_string_quote_choice_minimises_escapes` placeholder is now an **active** conformance test (`she said "hi"`→single-quote, `o'malley`→double, `plain`→double) — the follow-up re-port is done.

### gap-027 — Precedence-aware paren insertion not implemented

- **Status:** RESOLVED in CLOC12.10 (incidental, via the same fix as gap-024)
- **Upstream test:** `CodePrinterTest` operator-precedence lines (e.g. `a*(b+c)` keeps inner parens)
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** The precedence ladder added for gap-024 covers this directly. `emit_expression_inner(e, parent_prec)` checks `expr_prec(e) < parent_prec` and inserts parens when so — which means `a * (b + c)` correctly keeps the inner parens because `+` (prec 11) < `*` (prec 12). **CLOC12.138:** the `test_operator_precedence_inserts_inner_parens` placeholder is now an **active** conformance test (`a*(b+c)`, `(a+b)*c`, and `a+b*c` no-parens) — the follow-up re-port is done.

### gap-028 — VLQ encoder for source-map `mappings` field not implemented

- **Status:** RESOLVED in CLOC12.29 + CLOC12.31. **Step 1** (CLOC12.29) shipped the base64-VLQ primitives (`encode_vlq_int` / `encode_vlq_segment`) cross-checked against Mozilla's `source-map`, Google's `Base64VLQ.java`, and the v3 worked examples. **Step 2** (CLOC12.31) wires the primitives into `SourceMapBuilder::build()`: each pending `(generated_line, generated_column, cv_id)` is resolved to `(source_index, original_line, original_column)` via the `CVLog` — looking up the entry's own `Origin` first, then walking `parent_ids` depth-first with a cycle guard. `Origin.location` is parsed as `"line:col"`; non-matching free-form locations (e.g. `"row_id:N"`) fall through to the 1-field segment shape. The `sources` list is populated in first-seen order; `names` stays empty (5-field segments need a per-mapping name hint the emitter doesn't currently surface). The delta-encoded `mappings` string uses `;` between lines (with `generated_column` reset) and `,` between segments on the same line; leading semicolons cover the gap from line 0 to the first mapped line. 12 new inline tests pin: single 4-field segment, same-line delta encoding, multi-line `;` separators, two-source first-seen indexing, unresolved 1-field fallback, mixed resolved + unresolved, parent-chain origin lookup, defensive sort of out-of-order input, unparseable `location`, leading-`;` first-mapping-on-later-line, empty builder, self-referential cycle.
- **Upstream test:** `SourceMapGeneratorV3Test::testBasicMapping*`, `testLiteralMappings*`, `testMultilineMapping*`, `testMultiFunctionMapping`, `testGoldenOutput*` (almost the entire upstream file)
- **Ported file:** `closure-source-map/tests/upstream/source_map_generator_v3_test.rs`
- **Remaining:** The 7 `#[ignore]`-d upstream ports stay ignored. They exercise the full Closure-compiler pipeline (`compileAndCheck` driving `lex → parse → emit → source-map generate`) and assert against specific Closure-byte-identical VLQ strings; reaching them needs the closurec end-to-end harness plus golden capture from upstream. Tracked under CLOC14.1.

### gap-029 — identity-of-typeof-same-identifier fold not implemented

- **Status:** RESOLVED in CLOC12.17 (PR pending).
- **Upstream test:** `PeepholeFoldConstantsTest::testStringStringComparison` (`typeof a === typeof a` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution note:** Added a new structural-equality arm in `try_fold_binary_op` for `StrictEq`/`StrictNotEq` operators where both sides are `UnaryExpression { op: TypeOf, argument: Identifier }` with the same identifier name. Folds to `true`/`false` respectively. Identifier-only because `typeof <undeclared>` is special-cased by ECMAScript §UnaryTypeofExpression to return `"undefined"` rather than throw, so the fold is safe even without declaration-tracking — `typeof x` evaluated twice deterministically produces the same string. Member/call expressions are deliberately NOT folded because they can have side effects that we can't prove are absent without a heavier purity analysis. `test_typeof_identifier_identity_fold` un-ignored.

### gap-030 — function-declaration semicolon ASI policy

- **Status:** **FULLY RESOLVED** by CLOC12.38 (AST emitter half — merged PR #5140) and CLOC12.39 (CLI WHITESPACE_ONLY token re-stitcher half, PR pending). The `minify_function_decl` seed fixture flipped from IGNORED to **PASS** in CLOC12.39: closurec now emits `function f(){return 1};\n` byte-for-byte identical to upstream Closure v20240317.
- **Upstream byte-identity test:** `minify_function_decl` seed fixture (in CLOC14 harness). PASS.
- **The two halves:**

  **A. AST emitter side (closure-emitter)** — **RESOLVED in CLOC12.38** (merged PR #5140).
  - `emit_block_statement` drops a trailing `;` before `}` via `pop_trailing_semi_if_compact()` (compact mode only), gated by `last_stmt_uses_terminator_semi` so EmptyStatement-body cases like `if(x);` survive.
  - `emit_function_declaration` emits `;` after the body's closing `}` in compact mode.
  - Pretty mode is intentionally untouched.

  **B. CLI WHITESPACE_ONLY token re-stitcher side (`closurec/src/whitespace_only.rs`)** — **RESOLVED in CLOC12.39.**
  - The closurec CLI uses a token-level re-stitcher (NOT closure-emitter) for WHITESPACE_ONLY. CLOC12.39 ports the same two rules to this layer:
    1. **Rule A — drop `;` token before `}`**: with a `body_position_next` guard that protects EmptyStatement body slots (`if(x);` / `while(x);` / `for(;;);`).
    2. **Rule B — emit `;` after `}` of a function DECLARATION body**: state machine tracks "saw `function` keyword at statement boundary" + a per-`{` brace stack flag.
    3. **Rule C — dedup**: source `;` immediately after a synthetic `;` (rule B) is dropped to avoid `};;` in output for shapes like `function f(){};var g=1;`.
  - 10 inline tests pin every rule + edge case (function-expression doesn't get trailing `;`, if/while/for body slots preserved, dedup works, multi-stmt drops only last `;`, top-level `var` unchanged).
  - The pre-existing `tests/diff/whitespace-only/expected.stdout` was updated — its hand-written golden predicted `function add(a,b){return a+b;}` (closurec's old shape), but upstream Closure actually emits `function add(a,b){return a+b};`. Re-captured from upstream JAR to match the now-correct output.

### gap-031 — empty `{}` body collapses to `;`

- **Status:** **RESOLVED** in CLOC12.41 (PR pending). `minify_for_loop` flipped IGNORED → PASS.
- **Upstream byte-identity test:** `minify_for_loop` seed fixture. PASS.
- **Resolution:** Added a third gap-rule in `whitespace_only.rs` between rule A (drop `;` before `}`) and the token emit. When the current token is `{` AND `body_position_next` is true AND the next non-trivia token is `}`, emit a single `;` and skip both braces. The substitution is ECMAScript §13.2 — either Block or EmptyStatement satisfies the body-position Statement nonterminal. The `body_position_next` guard is critical: it scopes the rule to true body-positions (for/while/if/labeled) and leaves untouched (a) function-decl bodies (no control-flow paren-stack push, so guard is false), (b) plain Block-as-statement at top level (no guard), (c) object literals in expression position (no guard), and (d) try/catch bodies (try doesn't have a `(...)` head, so guard is false). 8 inline tests pin the behaviour including 5 non-regression tests for the cases the guard must NOT fire on.
- **Original divergence (now historical):**
  - Input: `for(var i=0;i<10;i++){}`
  - Upstream: `for(var i=0;i<10;i++);`
  - closurec: `for(var i=0;i<10;i++){}`
- **Applies to:** Empty `{}` in any control-flow body position: `for(...){}`, `while(x){}`, `if(x){}`, `if(x){}else{}` (each else position), `for(x in y){}`, `for(x of y){}`.
- **What it needs:** CLI token-state-machine extension. When `body_position_next` is true AND the next two tokens are `{` and `}` (in that order), emit `;` instead of `{}`. Pretty mode keeps `{}` for readability. Per-language, ECMAScript §13.4 (EmptyStatement) is exactly the substitution upstream is doing.

### gap-032 — single-statement if/else block flattening (CLI)

- **Status:** **RESOLVED** in CLOC12.42 (PR pending). `minify_if_else` flipped IGNORED → PASS. **The byte-identity harness now reports 17 matched, 0 failed, 0 skipped (of 17 total)** — the entire 17-fixture seed set is byte-for-byte identical to upstream.
- **Upstream byte-identity test:** `minify_if_else` seed fixture. PASS.
- **Resolution:** Option (b) chosen — a CLI-only token-level rule. When the re-stitcher encounters `{` in body position, it forward-scans for the matching `}` collecting eligibility info: must have exactly 1 `;` at depth 0, no nested `{`, no `function`/`try`/`if`/`while`/`for`/`do`/`switch`/`class` keyword at depth 0, and the last token before the close-`}` must be `;`. When eligible, the inner content is pre-emitted directly (bypassing the main loop), then both braces are skipped. Also armed: `else` keyword now sets `body_position_next = true`, so the else-clause body becomes a flatten target too. 13 inline tests pin the rule + non-regressions. Pre-push security review traced 6 concerns (dangling-else, string literals containing structural chars, regex literals, prev_emitted_tok, paren_stack/brace_stack invariants) — verdict PASS.

### gap-033 — try/catch trailing `;` after `}`

- **Status:** **RESOLVED** in CLOC12.40 (PR pending). `minify_try_catch` flipped IGNORED → PASS.
- **Upstream byte-identity test:** `minify_try_catch` seed fixture. PASS.
- **Why it failed:** Upstream emits a `;` after the last `}` of a try/catch statement, mirroring its function-decl normalisation. closurec correctly dropped inner `;`s per gap-030 rule A but never emitted the trailing `;`.
- **Resolution:** Extended the CLI token state machine. `brace_stack` was refactored from `Vec<bool>` to `Vec<BlockKind>` with three variants: `Function`, `TryChain`, `Other`. A new flag `next_block_is_try_chain` is armed by the `try`/`catch`/`finally` keywords; the next `{` consumes it and pushes `BlockKind::TryChain` onto the stack. When a `}` pops a `TryChain` kind, the emitter peeks the next non-trivia token: if it's `catch` or `finally`, the chain continues and no `;` is emitted; otherwise the chain has ended and a synthetic `;` is appended. 6 inline tests pin the behavior including nested try/catch, try/catch/finally chains, function-decl inside try-block (no interference between Function and TryChain), and ES2019 optional catch binding (`try{a;}catch{b;}`).

### gap-034 — class declaration trailing `;` after `}`

- **Status:** **RESOLVED** in CLOC12.43 (PR pending). `minify_class` flipped IGNORED → PASS. **The byte-identity harness now reports 25/25 PASS across the expanded seed set.**
- **Resolution:** Added `BlockKind::Class` variant. `class` keyword at statement boundary arms `saw_class_kw_at_boundary`; next `{` consumes it. On matching `}`, emit synthetic `;`. **Critical bug caught by pre-push security review**: the initial cut armed the flag whenever `class` appeared at a statement boundary, contaminating the next unrelated `{` when `class` was used as an object-literal property name (`var o={class:1};do{y}while(x);` would emit `do{y};while(x);` — a SyntaxError). Same defect family as gap-033's `try`-as-property bug. Fix: also require the next non-trivia token to look like a class-decl continuation (`{`, `extends`, or an identifier — NOT `:`/`,`/`;`/`}`/`)`/`]`/`.`/`=`/`(`).

### gap-035 — `var{...}` / `let{...}` / `const{...}` destructuring requires space before `{`

- **Status:** **RESOLVED** in CLOC12.43 (PR pending). `minify_destructuring` flipped IGNORED → PASS.
- **Resolution:** Extended `needs_separator` with a 3-keyword whitelist: when prev is `var`/`let`/`const` AND next is `{`/`[`, force a separator. Keeps the change scoped; doesn't affect general PUNCTUATION-after-KEYWORD shapes.

### gap-036 — switch statement trailing `;` after `}`

- **Status:** **RESOLVED** in CLOC12.43 (PR pending). `minify_switch` flipped IGNORED → PASS.
- **Resolution:** Added `BlockKind::Switch` variant + parallel `paren_is_switch_stack`. `switch` keyword (when followed by `(`) arms `next_paren_is_switch_head`. The matching `)` arms `next_block_is_switch_body`. The next `{` pushes `BlockKind::Switch`. On matching `}`, emit synthetic `;`. **Pre-push security review flagged a parallel non-fatal defect** for `switch` as a property name — same family as the `class` bug but only emits cosmetic extra `;`s rather than SyntaxErrors. Fix: require next token to be `(` (grammatically mandatory per §13.12).

### gap-037 — async function declaration trailing `;`

- **Status:** **RESOLVED** in CLOC12.44 (PR pending). `minify_async` flipped IGNORED → PASS.
- **Resolution:** Added `saw_async_kw_at_boundary` flag. `async` keyword at stmt boundary arms it ONLY when the very next non-trivia token is `function` (filters out async-arrow `async()=>x`, async method shorthand `{async m(){}}`, etc.). The next `function` keyword checks both `at_stmt_boundary` and `saw_async_kw_at_boundary` to decide whether to arm `saw_function_kw_at_boundary`. Same guard family as gap-033's `try` and gap-034's `class` — never arm a keyword flag without next-token confirmation.

### gap-038 — hex numeric literal normalised to decimal

- **Status:** **RESOLVED** in CLOC12.45 (PR pending). `minify_hex_number` flipped IGNORED → PASS. **Harness reports 33/33 PASS — full byte-identity across the entire seed set.**
- **Resolution:** Added `normalize_number_value()` helper to the WHITESPACE_ONLY token emit path. Detects hex/oct/bin integer literals via prefix (`0x`, `0X`, `0o`, `0O`, `0b`, `0B`), parses to `u128`, formats as decimal, emits whichever is shorter (tie-break to decimal). Verified against `closure-compiler-v20240317.jar` for ties — upstream's behaviour is "decimal when ≤ source length". `is_number_literal()` helper mirrors `is_string_literal()` for grammar-name detection.
- **Limitations carried forward** (each will become its own gap if a fixture surfaces it):
  - **BigInt literals** (`0xfn`) need arbitrary-precision arithmetic — left verbatim for now. Upstream would emit `15n`.
  - **Decimal floating-point shortest-form** (`0.5` → `.5`, `10.0` → `10`) is a different normalisation family handled elsewhere in upstream's code path.
  - **Scientific notation uppercasing** (`1e3` → `1E3`) is a separate rule — pure case change, not numeric normalisation.
  - **u128 overflow**: literals exceeding `u128::MAX` parse as `None` and stay verbatim rather than panicking.

### gap-039 — tagged template needs no separator between IDENT and `` ` ``

- **Status:** **RESOLVED** in CLOC12.46 (PR pending). `minify_tagged_template` flipped IGNORED → PASS. **Harness back to 41/41 PASS** (fourth 100% milestone today).
- **Resolution:** Added a short-circuit at the top of `needs_separator`: when the next token's value starts with `` ` ``, return false unconditionally. This filter runs BEFORE the word-like rule so any IDENT/keyword/number followed by a template literal emits no space — matching upstream's tagged-template grammar (§13.3.11) which forbids whitespace between the tag function and the template's opening backtick.

### gap-040 — numeric separator + scientific shortest-form

- **Status:** **RESOLVED** in CLOC12.48 (PR pending). `minify_numeric_separator` flipped IGNORED → PASS. **Harness now 49/49 PASS — fifth 100% milestone today** (after 17/17, 25/25, 33/33, 41/41).
- **Upstream byte-identity test:** `minify_numeric_separator` seed fixture.
- **Why it fails:** Upstream emits `var x=1E6;` for `var x=1_000_000;`. closurec preserves the underscored literal verbatim. This is TWO normalisations stacked:
  1. Strip ES2021 numeric separators (`_`) from the literal.
  2. Apply shortest-form: `1000000` (7 chars) vs `1E6` (3 chars) → scientific wins (decimal form falls back to scientific when shorter).
- **What it needs:** Extend `normalize_number_value()` (gap-038) to:
  - Strip `_` from the digit run before parsing (`u128::from_str_radix` doesn't accept separators).
  - After computing the decimal form, also compute the scientific form and pick the shortest of {source-form, decimal, scientific}, tie-breaking deterministically.
  - Uppercase `e` to `E` for the scientific form (verified by JAR probes: `1e3` → `1E3`).

### gap-041 — nested function-decl double synthetic `;`

- **Status:** **RESOLVED** in CLOC12.47 (PR pending). `minify_nested_function` flipped IGNORED → PASS. Harness now 48/49 (only gap-040 remains).
- **Resolution:** Implemented a `deferred_synthetic_semi` flag carried across iterations. When a `}` would emit a synthetic `;` but the next non-trivia is another `}`, the `;` is **deferred** to that outer brace instead of emitted. The outer brace consumes the deferred state (collapsing with any own-`;` it would emit). Chain continuations (`catch`/`finally`) carry the deferred state forward across the chain. Verified against `closure-compiler-v20240317.jar`: matches upstream for `function f(){function g(){}}` and `if(x){function f(){}}` and the try-chain composition `try{function f(){}}catch(e){b;}`.
- **Test expectations updated**: 4 pre-existing inline tests had encoded the buggy `function f(){function g(){};};` and similar outputs as their `assert_eq!` rhs. Updated to the correct upstream-matching form with comments referencing the JAR probe.

### gap-042 — `do` keyword should arm body_position_next

- **Status:** **RESOLVED** in CLOC12.49 (PR pending). `minify_do_while` flipped IGNORED → PASS. Harness 56/57 (only gap-043 left).
- **Resolution:** Added `else if val == "do"` branch arming `body_position_next = true`. Unlike `if`/`while`/`for` which arm via the `next_paren_is_control_flow_head` mechanism (their body opens after `)`), `do` opens its body slot IMMEDIATELY per §13.7.2. gap-032's single-statement flatten then fires correctly. 2 inline tests added; also documented a separate latent issue with empty-body do-while (`do{}while(x);` produces a spurious space between `;` and `while`) that's a `prev_emitted_tok` update bug in gap-031, orthogonal to this gap.
- **What it needs:** Add `do` to the keyword arm list (alongside `if`/`while`/`for`) — but note `do` arms body_position_next IMMEDIATELY (no following `(`), unlike the others. Insert at the right `else if val == "do"` branch arming `body_position_next = true`.

### gap-043 — CLI quote-choice optimisation

- **Status:** **RESOLVED** in CLOC12.50 (PR pending). `minify_escape_chars` flipped IGNORED → PASS. **Harness back to 57/57 — sixth 100% milestone today** (after 17/17, 25/25, 33/33, 41/41, 49/49).
- **Upstream byte-identity test:** `minify_escape_chars` seed fixture.
- **Why it fails:** Upstream switches between `"` and `'` based on which yields a shorter output (escapes fewer chars). closurec's CLI WHITESPACE_ONLY path always uses `"` via `push_quoted_string_content`. The AST emitter already has this logic (gap-026 closed in CLOC12.11), but the CLI doesn't go through the AST.
- **What it needs:** Lift `pick_better_quote` and `push_quoted_string_content` logic from closure-emitter into a shared module (or duplicate it carefully) and call it from `whitespace_only.rs`. Counting rule: prefer the quote style that requires fewer escape sequences; tie-break to the source-form's quote.

### gap-044 — JavaScript lexer does not support template literal substitution `${...}`

- **Status:** **RESOLVED (first slice)** in F10 lexer mode work (merged). Simple-identifier substitutions (`${name}`, `${x}`) lex and emit correctly via the `TEMPLATE_HEAD`/`TEMPLATE_MIDDLE`/`TEMPLATE_TAIL` declarative mode transitions. `minify_template_subst` and `minify_tagged_subst` fixtures both pass. **Residual:** expressions with operators (`.`, `+`, `(`, …) or nested `{}` inside `${…}` trip the div/default mode reset, losing template context and raising a `LexerError`. See gap-044b for the follow-up.
- **Upstream byte-identity test:** `minify_template_subst` (`\`hello ${name}\``) and `minify_tagged_subst` (`tag\`hi ${name}\``) — both PASS.

### gap-044b — template substitution with non-identifier expressions (`${obj.name}`, `${a+b}`)

- **Status:** OPEN. Discovered as the residual of gap-044 (see CLOC12.135).
- **Input examples:** `` `${obj.name}` ``, `` `${a + b}` ``, `` `${f()}` ``, `` `${x ? y : z}` ``
- **Why it fails:** The F10 declarative mode table transitions to `div` mode after a NAME token (to distinguish `/` from regex). When inside `${…}`, a `.` following the NAME causes the mode to reset to default, losing the `template` group override that makes `}` lex as `TEMPLATE_TAIL` rather than a plain `RBRACE`. The lexer then sees the closing backtick as an unexpected character.
- **What it needs:** Brace-depth tracking across template substitution boundaries. The lexer needs to know it's inside `${…}` at depth 0 so that `}` closes the substitution rather than any nested block `{}`. This requires either (a) a stack of lexer modes in `GrammarLexer` that template-entry/exit pushes and pops, or (b) a separate post-pass that re-stitches TEMPLATE segments after regular tokenisation. Approach (a) is the correct architecture per §12.8.6; it requires `GrammarLexer` to maintain an explicit mode stack rather than a single active mode.

### gap-045 — single-argument arrow function should drop enclosing parens

- **Status:** **RESOLVED** in CLOC12.51 (PR pending). `minify_arrow_async` flipped IGNORED → PASS. Harness now 79/81; only gap-044 (template substitution, lexer-level) remains open.
- **Upstream byte-identity test:** `minify_arrow_async` seed fixture.
- **Why it fails:** Upstream emits `var f=async x=>x+1;` for `var f=async(x)=>x+1;` (drops the parens). closurec preserves the source form `(x)`. The arrow-function grammar §15.3.1 permits single-identifier parameter without parens — upstream normalises to the parens-less form because it's shorter (saves 2 bytes).
- **What it needs:** A token-level pattern detector: when seeing `(`, IDENT, `)`, `=>`, peek ahead and if the shape matches a single-bare-identifier arrow head, drop the `(` and `)` tokens. Care must be taken NOT to drop parens around: (a) typed parameters (`(x: T)=>...` — TS only, but our lexer might emit them), (b) default values (`(x=1)=>...`), (c) rest parameters (`(...args)=>...`), (d) destructuring (`({x})=>...`), (e) zero arguments (`()=>...`). The eligibility test is "exactly one IDENT token between matching `(` and `)`, followed by `=>`".
- **Composition with async arrow**: `async(x)=>...` → `async x=>...` works identically; the `async` keyword doesn't affect the eligibility check.

### gap-046 — trailing comma in array/object literal dropped under WHITESPACE_ONLY

- **Status:** **RESOLVED (array case)** in CLOC12.52 (PR pending). `minify_trailing_array_comma` flipped IGNORED → PASS. Object-literal case deferred to a future `gap-046b` since `}` discrimination between block-close and object-literal-close requires brace_stack awareness.
- **Upstream byte-identity test:** `minify_trailing_array_comma` seed fixture.
- **Why it fails:** Upstream emits `var a=[1,2];` for `var a=[1,2,];` (trailing comma dropped). closurec preserves it. The trailing-comma form is grammatically valid (§13.2.4 Elision), but it's a byte saving to drop. Also applies to object literals (`{a:1,}` → `{a:1}`).
- **What it needs:** When emitting a `,` token, peek ahead. If the next non-trivia token is `]` (or `}` in an OBJECT-LITERAL position), suppress the `,`. The OBJECT-LITERAL position distinction matters because `,` before `}` of a block (`{stmt;}` ← never has `,`) vs an object literal (`{a:1,}` ← does) requires knowing the brace-stack kind. Easier alternative: just check `]` — that's the array case. Object case can be a follow-up.

### gap-047 — suppress synthetic `;` after function-decl `}` before statement-starting keyword

- **Status:** **RESOLVED** in CLOC12.53 (PR pending). `minify_multi_line_func` flipped IGNORED → PASS. Harness now 87/89 — **only gap-044 (lexer-level template substitution) remains open**.
- **Upstream byte-identity test:** `minify_multi_line_func` seed fixture.
- **Why it fails:** Upstream emits `function add(a,b){return a+b}var sum=add(2,3);` for a multi-line input. closurec emits `function add(a,b){return a+b};var sum=add(2,3);` — the gap-030 synthetic `;` after the function-decl `}` is unneeded because `var` (and other statement-starting keywords) can never grammatically fuse with the preceding `}`. ASI safety doesn't require the `;`.
- **What it needs:** Extend the gap-030 trailing-`;` rule (and its gap-041 deferred-`;` cousin) with a peek-ahead suppression: if the next non-trivia token is a statement-starting keyword (`var`, `let`, `const`, `function`, `class`, `if`, `for`, `while`, `do`, `switch`, `try`, `return`, `throw`, `break`, `continue`), suppress the synthetic `;`. EOF stays at the SOURCE EOF behaviour (gap-030 still fires there).

### gap-048 — BigInt with numeric separator: strip `_` separators

- **Status:** **RESOLVED** in CLOC12.55 (PR pending). `minify_bigint_separator` flipped IGNORED → PASS.
- **Upstream byte-identity test:** `minify_bigint_separator` seed fixture.
- **Input:** `var a = 1_000_000n;`
- **Upstream:** `var a=1000000n;` (separators stripped, BigInt suffix kept)
- **Why it failed:** `is_number_literal` did not recognize the lexer's `BIGINT` token type, so BigInt tokens never reached `normalize_number_value` — gap-040's separator-stripping never fired on them.
- **Fix:** (1) Extend `is_number_literal` to accept `BIGINT` / `BIGINT_LITERAL` type-names. (2) In `normalize_number_value`, when the token ends with `n`, strip `_` from the body and re-append `n` (skip the radix-and-shortest-form path — that needs bigint arithmetic and is still deferred).

### gap-049 — flattened single-stmt for-body keeps trailing `;` before `}`

- **Status:** **RESOLVED** in CLOC12.56 + pinned by `minify_for_body_inner_close` fixture in CLOC12.134.
- **Upstream byte-identity test:** `minify_for_body_inner_close` (canonical gap-049 repro).
- **Why it failed:** gap-032's flatten emitted content `(idx+1)..close_idx` verbatim, which always includes the trailing `;`. When the next token after the closing `}` was itself a `}`, that `;` became redundant — Rule A would have dropped a source `;` at that position, but Rule A doesn't re-scan pre-emitted content.
- **Fix (CLOC12.56):** In gap-032's eligible branch, peek `kept.get(close_idx + 1)`. If it equals `}`, set `emit_end = close_idx - 1` (exclude the trailing `;`). The eligibility check already verified `last_before_close == ";"`, so this index is always the redundant `;`.
- **Note:** The gap was discovered by CLOC14.13 and initially listed as OPEN; the fix already lived in the code from CLOC12.56 (`drop_trailing_semi` logic). CLOC12.134 adds the `minify_for_body_inner_close` fixture that pins this invariant and removes the duplicate OPEN entry.

### gap-050 — `new X()` with empty arg list drops parens

- **Status:** **RESOLVED** in CLOC12.57 (PR pending). `minify_new_expr` flips IGNORED → PASS.
- **Upstream byte-identity test:** `minify_new_expr` seed fixture.
- **Input:** `var x = new Foo();`
- **Upstream:** `var x=new Foo;` (parens stripped)
- **Why it failed:** closurec's whitespace_only re-stitcher passed `(` and `)` through verbatim for `new Foo()`.
- **Fix:** Token-level peephole at `(` — when `kept[idx-2] == "new"` AND `kept[idx-1]` is a simple identifier AND `kept[idx+1] == ")"` AND `kept[idx+2]` is none of {`(`, `.`, `[`, `` ` ``}, skip both tokens. The forbidden followers bind tighter than NewExpression (member access, chained call, tagged template) — dropping `()` before them would change parse precedence. All looser-binding tokens (`;`, `}`, `,`, `+`, `instanceof`, etc.) are safe.

### gap-046b — object-literal trailing comma drop

- **Status:** **RESOLVED** in CLOC12.58 (PR pending). `minify_trailing_obj_comma` fixture matches byte-for-byte.
- **Upstream byte-identity test:** `minify_trailing_obj_comma` seed fixture.
- **Input:** `var o={a:1,b:2,};`
- **Upstream:** `var o={a:1,b:2};`
- **Why it failed:** closurec's whitespace_only re-stitcher passed `,` through verbatim.
- **Fix:** Mirror gap-046's `,`-before-`]` peephole for `,`-before-`}`. Brace_stack discrimination is NOT needed: in valid ECMAScript, `,` immediately before `}` can only appear in object-literal / object-destructuring contexts. Block bodies separate statements with `;`, class bodies have no separator, switch/try have no comma. So the drop is safe unconditionally.

### gap-051 — IIFE paren normalization

- **Status:** **RESOLVED** in CLOC12.60 (PR pending). `minify_fn_expr_iife` flips IGNORED → PASS.
- **Upstream byte-identity test:** `minify_fn_expr_iife` seed fixture.
- **Input:** `(function(){return 42;}());`
- **Upstream:** `(function(){return 42})();` (call moves outside the wrapping parens)
- **Fix:** Token-stream pre-pass: scan kept for the 4-token sequence `} ( ) )` and rotate `[i+1..=i+3]` right by 1 to reorder to `} ) ( )`. Safe-by-construction — this token sequence can only appear in IIFE contexts in valid JS.

### gap-052 — trailing `;` after `}` at EOF for control-flow / label-block bodies

- **Status:** **RESOLVED** in CLOC12.61 (PR pending). `minify_labeled_block` and `minify_double_break_continue` flip IGNORED → PASS.
- **Input:** `for(;;){if(x)break;if(y)continue;use();}` and `foo:{a();break foo;b();}`
- **Upstream:** Same with trailing `;`.
- **Fix:** Change `BlockKind::Other => false` in `kind_wants_semi` to `BlockKind::Other => next_val.is_none()`. EOF-only — mid-stream Other-blocks still emit no `;`.

### gap-053 — paren elision around var-init RHS

- **Status:** **RESOLVED** in CLOC12.62 (PR pending). `minify_null_undef_compare` flips IGNORED → PASS.
- **Input:** `var t = (x == null);`
- **Upstream:** `var t=x==null;` (outer parens stripped)
- **Fix:** Token-stream pre-pass: scan for `= ( ... )` where contents have no `,` at depth 0 and don't start with `function`. If token after matching `)` is `;`, `,`, or EOF, drop both `(` and `)`.

### gap-054 — paren elision around unary operand

- **Status:** **PARTIAL** — RESOLVED in CLOC12.63 for single-token operands. Multi-token member expressions (`delete(o.x)`) deferred.
- **Input:** `void(0);`
- **Upstream:** `void 0;` (parens stripped)
- **Fix:** Token-stream pre-pass: when prev is `void`/`typeof`/`delete` and next is `( SINGLE )` with SINGLE being one safe token (ident/num/string), drop both parens.

### gap-055 — paren elision around ternary arms (single-expression)

- **Status:** **RESOLVED** in CLOC12.64. Discovered by CLOC14.25 AFTER 3 consecutive zero rounds (marathon was NOT converged). `minify_ternary_assign` flips IGNORED → PASS.
- **Input:** `var r = x>0?(a=1):(b=2);` → **Upstream:** `var r=x>0?a=1:b=2;`
- **Fix:** Token-stream pre-pass: when prev is `?` or `:` and next is `(`, scan to matching `)`. Drop both parens iff the token after `)` is an arm-terminator (`:`/`;`/`,`/`)`/`]`/`}`/EOF — parens span the WHOLE arm) and no top-level `,` inside. Whole-arm guard prevents precedence shifts (`x?(a=1)+2:c` stays). `?.` lexes as single OPTIONAL_CHAIN token so optional calls are safe.

### gap-056 — paren elision after `return` / `throw` / `=>` (RESOLVED)

- **Status:** **RESOLVED** in CLOC12.65 (PR pending). Extends the gap-055 whole-arm pre-pass with prefixes `=>`, `return`, `throw`.
- **Guards:** property-name guard (don't strip `gen.throw((e))` / `it.return((b))`); arrow-brace guard (don't strip `()=>({a:1})` — block ambiguity). All token checks via `is_structural_punct`.
- **Note:** the inner-redundant-paren strip `gen.throw((e))`→`gen.throw(e)` is a SEPARATE future gap (nested grouping parens in call args), not gap-056.

### gap-057 — paren elision around member-expression object

- **Status:** **RESOLVED** in CLOC12.66. `minify_paren_then_member` flips IGNORED → PASS.
- **Input:** `var v = (a).b;` → **Upstream:** `var v=a.b;`
- **Fix:** Token-stream pre-pass: drop the grouping parens in the shape `GROUPING_PREFIX ( IDENT ) .`. Three guards make it provably safe — (1) GROUPING not CALL: the token before `(` must be a punctuation/operator other than `)`/`]`/`?.`, so call/index/optional-call parens (`f(a).b`, `a[i](b).c`, `x?.(a).b`) are never touched; (2) SINGLE PLAIN IDENTIFIER inside — numbers (`(1).toString()`), keywords, literals, regex are all excluded; (3) the token after `)` must be `.` (member position). A statement-leading `(` is harmless here because the single-identifier content can never be `{`/`function`, so there's no block/function ambiguity. `(a)[i]` and `(a)(x)` are safe too but deferred.

### gap-058 — numeric separator in float literals

- **Status:** **RESOLVED** in CLOC12.67. `minify_numeric_underscore_float` flips IGNORED → PASS.
- **Input:** `var x=1_000.5;` → **Upstream:** `var x=1000.5;`
- **Fix:** `normalize_number_value`'s float/scientific branch returned the value verbatim (leaving `_` in). It now returns `cleaned` (the value with every `_` already stripped) instead — separators removed for floats/scientific too. Full float shortest-form (`0.5` → `.5`, `1000e3` → `1E6`) remains a separate deferred gap; only the purely-lexical separator stripping is added here.

### gap-059 — member access on a `new` expression

- **Status:** **RESOLVED** in CLOC12.68 (minimal slice). `minify_new_member_chain` flips IGNORED → PASS.
- **Input:** `var x=new A().b;` → **Upstream:** `var x=(new A).b;`
- **Fix:** Token pre-pass that wraps the `new`-expression in parens when it's the object/callee of a following `.`/`[`/`(`. Implemented WITHOUT synthesising tokens: the empty arg-list `()` already provides a `(` and `)`, so the pass just REORDERS them — moves the `(` to before `new`, leaving the `)` after the identifier (`new A ( ) .` → `( new A ) .`). Guards: operator `new` only (not the property `.new`/`?.new`), single plain-identifier callee, empty arg-list, followed by `.`/`[`/`(`; all bracket checks via `is_structural_punct`. Complements gap-050 (which drops `new A()`→`new A` only when NOT followed by member/call). Verified against JAR for `.b`, `.b.c`, `[i]`, `()`, standalone, and property-`new`.
- **Deferred (follow-up):** member-callee `new a.b.C().d` → `(new a.b.C).d`, and arg-bearing `new A(y).b` → `(new A(y)).b`. The minimal slice handles single-identifier empty-arg new-expressions (the fixture); these broader shapes need callee-extent + arg-list scanning.

### gap-060 — member-callee new-expression

- **Status:** **RESOLVED** in CLOC12.70. `minify_new_member_callee` flips IGNORED → PASS.
- **Input:** `var x=new a.b.C().d;` → **Upstream:** `var x=(new a.b.C).d;`
- **Fix:** Generalized gap-059's new-expr reorder to a member-chain callee. The callee scan now consumes the leading identifier plus zero-or-more `.IDENT` accessors before the empty `()`, then reorders the `(` to before `new` (same no-synthetic-token trick). The single-identifier case (gap-059) is the zero-accessor special case, so the two are unified in one pass. Computed `[...]` callees and arg-bearing forms (gap-061) remain deferred. Standalone member-callee `new a.b.C()` → `new a.b.C` (no member follows) is a SEPARATE gap-050 limitation (gap-050 only handles single-identifier callees), not gap-060.

### gap-061 — arg-bearing new-expression member

- **Status:** **RESOLVED** in CLOC12.71. `minify_new_with_args_member` flips IGNORED → PASS.
- **Input:** `var x=new A(y).b;` → **Upstream:** `var x=(new A(y)).b;`
- **Fix:** A new pre-pass that, unlike gap-059/060, INSERTS synthetic parens (the non-empty arg list has no spare parens to reorder). Two synthetic grouping tokens — one `(` and one `)` — are cloned from the source's own parens and declared before `kept` so they outlive it; the pass inserts `&`-references to them (a `(` before `new`, a `)` after the arg-list's depth-balanced close). Reuses the gap-060 callee scan; handles member-chain callees, multiple args, and nested-call args (`new A(f(x)).b`). Guards: operator `new` only; non-empty args (empty is gap-059/060's reorder); follower ∈ `.`/`[`/`(`; all checks via `is_structural_punct`. This completes the new-expression-member family (gap-059/060/061).

### gap-062 — redundant double-paren collapse

- **Status:** **RESOLVED** in CLOC12.69 (minimal slice). `minify_double_paren_arith` flips IGNORED → PASS.
- **Input:** `var x=((a+b))*c;` → **Upstream:** `var x=(a+b)*c;`
- **Fix:** Token pre-pass — when a GROUPING `(` is directly followed by another `(` and the inner group's matching `)` is directly followed by the outer `)` (purely-nested `(( ... ))`), drop the outer pair. Guards: the outer `(` must be a grouping paren (prev is punct other than `)`/`]`/`?.`, or start) so a CALL paren like `f((a,b))` is never collapsed to `f(a,b)`; no top-level comma inside; all bracket checks via `is_structural_punct`.
- **Deferred (follow-up):** upstream eliminates parens far more aggressively — `((a))` → `a`, `(a)+(b)` → `a+b`, `f((a))` → `f(a)`. This slice strips only ONE directly-nested grouping layer; the broader redundant-paren pass is future work. (Note: `((a,b))` standalone is already correctly handled by gap-053's var-init paren elision.)

### gap-063 — same-sign `+`/`-` token adjacency (CORRECTNESS)

- **Status:** **RESOLVED** in CLOC12.72. `minify_neg_neg` flips IGNORED → PASS.
- **Input:** `var x=- -a;` → **Upstream:** `var x=- -a;` → **closurec (was WRONG):** `var x=--a;`
- **Severity:** This is a **semantic-corruption** bug, not a formatting nicety. The re-stitcher joins two adjacent tokens that both begin with `+` (or both with `-`), forming a spurious compound operator: `- -a` (negate the negation of `a`) becomes `--a` (pre-decrement of `a`) — a different program. Characterized cases (all currently WRONG in closurec):
  - `- -a` → `--a`, `+ +a` → `++a`
  - `a- -b` → `a--b`, `a+ +b` → `a++b`
  - `- --a` → `---a`, `+ ++a` → `+++a`, `a- --b` → `a---b`
  - Correct (already OK): `a+ -b` → `a+-b` (different signs — `+-` is unambiguous).
- **Fix (CLOC12.72):** `needs_separator()` gains a same-sign rule — insert a space when the previous token's LAST char and the next token's FIRST char are both `+`, or both `-`. **CRITICAL GUARD:** both sides must be real punctuator tokens (`is_punct`), never string/regex/template literals. A one-char string `"-"` stores `.value == "-"` (delimiters stripped), so without the `is_punct` gate a string ending in `-` (`"a-"`) followed by a `-` operator — emitted as `"a-"-…` where the char before the operator is the closing quote — would wrongly get a space. Verified: `"a-"-1` stays `"a-"-1` and `"a-"- -b` spaces only between the two real `-` operators. 6 unit tests added.

### gap-064 — string `)` argument misread as empty-paren close (CORRECTNESS)

- **Status:** **RESOLVED** in CLOC12.73. `minify_new_str_paren_arg` + `minify_new_str_paren_member` flip IGNORED → PASS.
- **Input:** `var z=new A(")");` → **Upstream:** `var z=new A(")");` → **closurec (was WRONG):** `var z=new A);`
- **Severity:** **Semantic corruption producing invalid JS.** The gap-050 `new X()` → `new X` empty-paren-drop pass checks `kept[idx+1].value == ")"` at `code/programs/rust/closurec/src/whitespace_only.rs:976` **without** the `is_structural_punct` guard. A string argument whose content is `)` stores `.value == ")"` (the lexer strips delimiters), so the pass mistakes the string for the empty-arg close paren — dropping the `(` and the string, leaving a stray real `)`. Second manifestation: `new A(")").b` → `(new A)).b` (broken). Plain calls (`f(")")`) are unaffected — only the `new`-expr empty-paren path has the unguarded check.
- **What it needs (CLOC12.73):** Change line 976 from `kept.get(idx + 1).map(|t| t.value.as_str()) == Some(")")` to `kept.get(idx + 1).map(|t| is_structural_punct(t, ")")).unwrap_or(false)`. Audit the sibling `.value == "("/")"` checks in the same region (lines ~218/324/968-971, the arrow-elision `kept[idx+2/3].value` checks at ~991-992) for the same latent bug and gate them on `is_structural_punct` too.

### gap-065 — redundant parens around a call / tagged-template callee

- **Status:** **RESOLVED** in CLOC12.74. `minify_paren_call_callee` + `minify_paren_member_callee` + `minify_paren_tagged_callee` flip IGNORED → PASS.
- **Input:** `(f)(x);` → **Upstream:** `f(x);` (also `(a.b)(x)` → `a.b(x)`, `` (f)`t` `` → `` f`t` ``)
- **What it needs:** Strip redundant parens around the CALLEE of a `CallExpression` / `TaggedTemplateExpression` when the inner expression is a *simple reference* — a bare identifier or a member-access chain (`.`/`[]`/`?.`). **Boundary (must NOT strip):** a sequence-expr callee `(a,b)(x)` keeps its parens (comma binds looser than call); any inner expression of lower precedence than the call/member would change meaning if unwrapped. Token-level: when a `(` is at an expression-callee position, its inner group is a plain reference chain, and the matching `)` is immediately followed by `(`/`` ` `` (call/tag), drop the paren pair. All bracket checks via `is_structural_punct`.

### gap-066 — redundant parens after `extends`

- **Status:** **RESOLVED** in CLOC12.75 (minimal safe slice). `minify_class_extends_paren` flips IGNORED → PASS.
- **Input:** `class A extends(B){}` → **Upstream:** `class A extends B{}`
- **What it needs:** After the `extends` keyword, a parenthesized simple reference (`(B)`) is redundant — strip the parens. Same simple-reference / precedence caveat as gap-065.
- **CLOC12.75 (minimal safe slice):** strips `extends ( <identifier-dot chain> )` only (identifier + `.IDENT` accessors), anchored on the `extends` KEYWORD (guarded against a property named `extends` — `o.extends(x)` is a method call). **Deliberately conservative vs upstream:** `extends(B||C)` is KEPT — `B||C` is not a LeftHandSideExpression, so `extends B||C` would be invalid JS (upstream strips it anyway, emitting arguably-invalid output). Call-chain inners (`extends(f())`) deferred. 5 unit tests.

### Additional divergences observed by CLOC14.33 (deferred, not yet fixtured)

- `new(f)()` → upstream `new f` — parens around a `new` callee plus empty-paren drop (a paren-elision + gap-050 composition). Deferred.
- `label:{break label}` → upstream `label:break label;` — labeled single-statement block flattening (block-flatten in a labeled-statement context). Deferred.

### gap-067 — labeled single-statement block flatten

- **Status:** **RESOLVED** in CLOC12.77 (provably-safe minimal slice). `minify_label_block_flatten` flips IGNORED → PASS.
- **Input:** `label:{break label}` → **Upstream:** `label:break label;`
- **What it needs:** When a `LabeledStatement`'s body is a `BlockStatement` containing exactly ONE statement, the braces are redundant — flatten `label:{S}` → `label:S`. A multi-statement block keeps its braces (`label:{a();break label}` stays — pinned by `minify_label_block_multi`). Same family as gap-010/032 (block flattening) but in a labeled-statement context.
- **CLOC12.77 (provably-safe slice):** flattens `IDENT : { <completion-keyword> … }` where the label sits at a hard statement boundary (prev token is `;`/`}`/start — `{` is EXCLUDED, since a `{`-preceded `IDENT:{…}` is an object-literal value like `{x:{break:1}}`), the body's first token is `break`/`continue`/`return`/`throw` (unambiguously a statement — never an object value or a declaration), and it is a single statement. The `{` is dropped and the `}` becomes the terminating `;` (a synthetic `;` is injected when the body had no trailing `;`). **Conservative trade-off:** nested labels (`outer:{inner:{break outer}}`) only flatten the innermost (a missed optimization, not a corruption); expression-statement and `var`-declaration bodies are deferred. 5 unit tests; object-literal/ternary safety verified.

### gap-068 — redundant parens around a `new` callee

- **Status:** **RESOLVED** in CLOC12.76. `minify_new_paren_callee` + `minify_new_paren_member` flip IGNORED → PASS.
- **Input:** `new(f)();` → **Upstream:** `new f;` (also `new(a.b);` → `new a.b;`)
- **What it needs:** Strip the grouping parens around the CALLEE of a `NewExpression` when the callee is a simple reference (identifier or member chain). For the call form `new(f)()` this composes with the gap-050 empty-paren drop (`new f()` → `new f`). The simple-reference / precedence caveat from gap-065 applies — an operator inner (`new(a+b)`) must keep its parens.
- **CLOC12.76:** strips `new ( <identifier-dot chain> )`, anchored on the `new` KEYWORD (guarded against a property named `new` — `o.new(f)` is a method call). The trailing empty `()` of the call form is dropped by gap-050 in the emit loop. Operator inner `new(a+b)` keeps its parens (would parse as `(new a)+b`). **Note — separate pre-existing divergence (gap-069 candidate):** when the parens are KEPT, upstream emits a space `new (a+b)` while closurec emits `new(a+b)`; both are valid and equivalent, but not byte-identical. Deferred. 5 unit tests.

### gap-069 — `new(` emit-adjacency space

- **Status:** RESOLVED in CLOC12.78. `minify_new_paren_space` enforced.
- **Input:** `new(a+b);` → **Upstream:** `new (a+b);` (also `new(a,b)` → `new (a,b)`)
- **What it needed:** When the `new` keyword is directly followed by a `(` GROUPING paren (a parenthesized callee expression that gap-068 keeps), the re-stitcher inserts a single space: `new (…)`. Both forms keep the parens; this is an emit-adjacency rule for `new` + `(`. (Noted as the gap-069 candidate during CLOC12.76.)
- **CLOC12.78 resolution:** A two-token look-behind, `new_paren_needs_space(kept, idx)`, consulted at the main emit site (NOT in `needs_separator`, which sees only the adjacent pair). Distinguishing the genuine NewExpression keyword `new` from a PROPERTY named `new` (`o.new(f)` — a method call) requires the token *before* `new`: the JS lexer is context-free and types `new` identically in both, so only a preceding `.`/`?.` member accessor tells them apart. The helper fires only when (a) `kept[idx]` is a structural `(`, (b) `kept[idx-1]` is the word-like `new` keyword, and (c) `kept[idx-2]` (if any) is not a `.`/`?.` accessor. The companion `new(f)()` simple-reference form never reaches here — gap-068's pre-pass has already elided those parens to `new f`. 3 unit tests + the byte-identity fixture.

### gap-070 — `delete` operand member-chain paren elision

- **Status:** RESOLVED in CLOC12.79. `minify_delete_paren_elide` enforced.
- **Input:** `delete(a.b)` → `delete a.b` (also `delete(a.b.c)`, `delete(a[b])`).
- **What it needed:** Strip the redundant grouping parens around the OPERAND of the `delete` prefix operator when the operand is a member-reference chain — exactly the shape `typeof`/`void` already handled for single tokens.
- **CLOC12.79 resolution:** The existing gap-054 pre-pass (which handled `void`/`typeof`/`delete` for a single safe token) was generalised. The operand check now accepts a **member-reference chain** — an identifier base followed by any run of `.name` / `?.name` / `[…]` accessors with no top-level operator, call, or comma — via the new `is_safe_unary_operand` helper. Both shapes are higher-precedence than the unary operator and self-delimiting, so the parens are pure grouping (`OP(REF)` ≡ `OP REF`). The matching close paren is found by a structural depth scan rather than the old fixed `i+3` offset.
- **Correctness fix bundled in:** the pre-pass previously lacked a PROPERTY GUARD, so `o.delete(a)` (a Map/Set `.delete()` method call) mis-emitted as the **invalid** `o.delete a`. A `.`/`?.` look-behind now skips property-named keywords (`o.delete(`, `o.typeof(`, …). The same generalisation also closes `typeof(a.b)` / `void(a.b)` against the JAR. 4 unit tests + the byte-identity fixture.

### gap-071 — instanceof right-operand paren elision

- **Status:** RESOLVED in CLOC12.82. `minify_instanceof_paren` enforced.
- **Input:** `a instanceof(B)` → `a instanceof B` (also `a instanceof(b.c)`, `a instanceof(b[c])`).
- **What it needed:** Strip the redundant grouping parens around the RIGHT operand of the binary `instanceof` operator when the operand is a simple reference (single token or member chain).
- **CLOC12.82 resolution:** `instanceof` was added to the gap-054/070 unary-keyword paren-elision pre-pass keyword set (`void`/`typeof`/`delete`/`instanceof`). Although `instanceof` is a *binary* operator, the right-operand elision is mechanically identical to the prefix-unary cases — the left operand sits at `kept[i-1]` and is irrelevant to whether the right operand's grouping parens are redundant. `instanceof` binds looser than member access, so `a instanceof(B.c)` ≡ `a instanceof B.c` and whatever follows the close paren re-associates identically. The existing `is_safe_unary_operand` check (single token or member-reference chain) and property guard apply unchanged: operator operands (`a instanceof(B||C)`) keep their parens, and `o.instanceof(x)` (a property method call — `instanceof` reserved word as a property name) is skipped. 3 unit tests + the byte-identity fixture.

### gap-072 — await operand paren elision

- **Status:** RESOLVED in CLOC12.106. `minify_await_paren_elide` / `minify_await_binary_kept` enforced.
- **Input:** `await(x)` → `await x`.
- **What it needs:** Same simple-reference operand paren elision as gap-070/071, but for the `await` unary operator. Deferred: `await` is async-context-only, so a naive keyword-anchored elision could mis-handle a non-async `await` used as a plain identifier (`await` is only reserved inside async functions/modules). Needs an async-context guard before it can be added to the keyword set safely.
- **CLOC12.106 resolution:** The original async-context worry dissolved once verified against the JAR: the upstream compiler **rejects** any non-async `await` as a PARSE ERROR ("await must be inside asynchronous function"), so identifier-`await` never appears in a byte-identity input — every accepted input has `await` as the operator. `await` binds at UNARY precedence (like `typeof`/`void`/`delete`), so it was added to gap-101's `is_safe_unary_kw_operand` keyword block (NOT the gap-056 `yield` block, which is for the looser-binding `yield`). A safe operand drops its parens; a parenthesised BINARY operand keeps them (`await` binds tighter than the binary op). Two extra concerns: (1) **always-space** — upstream emits the operator with a separating space before its operand even when non-word-like (`await -b`, `await (a+b)`), handled by a new `await_operator_needs_space` emit predicate; (2) **contextual-keyword guards** — `await` as a function/method NAME (`function await(x){}`, `{await(x){}}` — matched `)` followed by `{`) or a property (`o.await(x)`) is excluded from both the drop and the space. JAR-verified across all operand kinds + the name/property guards; +3 `gap072_*` unit tests. Known residual: deeply-nested `await(await(x))` keeps the inner parens (the keyword block does not recurse into a dropped span — a pre-existing pattern shared with the other keywords).

### gap-073 — `get`/`set` before a computed key needs a space

- **Status:** RESOLVED in CLOC12.80. `minify_get_computed_space` enforced.
- **Input:** `var o={get[k](){return 1}};` → **Upstream:** `var o={get [k](){return 1}};` (also `set[k](v){}`).
- **What it needed:** When a `get`/`set` ACCESSOR keyword is directly followed by a COMPUTED key `[`, insert a space — `get [k]` — otherwise `get[k]` re-reads as a member access on a variable named `get` rather than a getter with a computed name.
- **CLOC12.80 resolution:** A two-token-look-behind + forward-check helper `get_set_computed_needs_space(kept, idx)`, consulted at the main emit site (NOT in `needs_separator`, which sees only the adjacent pair). `get`/`set` are *contextual* keywords (accessors only inside an object/class body, plain identifiers elsewhere) and the JS lexer types them identically, so disambiguating a real accessor from member access (`o.get[k]`) or variable indexing (`get[k](x)`) needs more context. The helper fires only when (a) `kept[idx]` is a structural `[`, (b) `kept[idx-1]` is the word-like `get`/`set` keyword, (c) `kept[idx-2]` is an object-literal property-start `{`/`,` (the decisive guard — excludes `.`/`?.` member access and statement-level indexing), and (d) the token after the matching `]` is a structural `(` (the accessor's parameter list). Verified against the JAR. Class-body accessors after a previous member (`}`-/`static`-preceded) are deferred. 2 unit tests + the byte-identity fixture.

### gap-074 — loop-body single-statement block flatten

- **Status:** RESOLVED in CLOC12.81. `minify_loop_body_flatten` enforced.
- **Input:** `l:for(;;){continue l}` → **Upstream:** `l:for(;;)continue l;` (also `for(;;){break}`, `while(x){g()}`, `for(a in o){h(a)}`, `for(a of o){h(a)}`).
- **What it needed:** When a loop body (`for`/`while`/`for-in`/`for-of`) is a single-statement block, flatten `for(…){S}` → `for(…)S;`. Loop-body sibling of gap-067 (labeled-block flatten).
- **CLOC12.81 resolution:** A pre-pass anchored on a `for`/`while` STATEMENT keyword (word-like, NOT a property — a `.`/`?.` look-behind disqualifies `o.while(x){…}` method calls). The header `(…)` is matched by a structural depth scan; the token after `)` must be a `{`. A `{` immediately following a loop header is UNAMBIGUOUSLY a loop body — never an object literal — so (unlike gap-067) no completion-keyword guard is needed. The body is dropped-braces + a synthetic `;` (reusing gap-067's `synth_semi`). Scoped to the provably-safe slice: the body has NO nested `{`, NO control-flow keyword at depth 1, and EXACTLY ZERO top-level `;` (a single un-terminated statement). Bodies that already end in `;` are left to the gap-032 emit-time flatten; multi-statement (`{a();b()}`), empty (`{}`), and nested-control-flow (`for(;;){if(x)a()}`) bodies keep their braces (deferred). Also deferred: `if`-body and `do…while`-body flatten (different anchors). 5 unit tests + the byte-identity fixture.

### gap-075 — prefix-unary symbol operand paren elision

- **Status:** RESOLVED in CLOC12.84. `minify_unary_minus_paren` enforced.
- **Inputs:** `-(a)` → `-a`; `!(a)` → `!a`; `~(a)` → `~a`; and the same-sign case `-(-a)` → `- -a`, `+(+a)` → `+ +a`.
- **What it needed:** Strip the redundant grouping parens around the operand of a SYMBOL `-`/`+`/`!`/`~` operator when the operand is a simple reference — the `is_safe_unary_operand` machinery (gap-070/071) but anchored on the punctuation operator (`is_structural_punct`-gated) rather than a word-like keyword.
- **CLOC12.84 resolution:** A new pre-pass anchored on `is_structural_punct(kept[i], "-"|"+"|"!"|"~")` with `kept[i+1]` a `(`. **Prefix-vs-binary turned out to be irrelevant:** stripping a grouping paren around a self-delimiting operand is sound whether the operator is a prefix unary (`-(a)` → `-a`) OR a binary operator whose RIGHT operand is parenthesised (`a-(b)` → `a-b`, which the JAR also does). The operand check is the new `is_safe_unary_paren_operand` — it accepts everything `is_safe_unary_operand` does PLUS a leading chain of prefix SYMBOL unaries applied to such an operand (`-a`, `!a`, `~a.b`), which is what makes `-(-a)`'s operand (itself a UnaryExpression) strippable. Operator operands (`-(a+b)`, `a-(b+c)`) are rejected and keep their parens. **Same-sign:** `--`/`++` (single tokens whose `.value` is `"--"`/`"++"`) never match the bare-`-`/`+` anchor, and the existing gap-063 `needs_separator` rule inserts the separating space (`- -a`) once the parens are gone. **Deferred:** the matching LEFT-operand elision (`(a)+(b)` → upstream `a+b`; closurec now reaches `(a)+b`), the predecrement-operand case `a-(--b)` → `a- --b`, and binary comparison operands (`a!=(b)` → `a!=b`) are separate gaps. 3 unit tests + the byte-identity fixture.

### gap-076 — with-body single-statement block flatten

- **Status:** RESOLVED in CLOC12.83. `minify_with_body_flatten` enforced.
- **Input:** `with(o){a()}` → **Upstream:** `with(o)a();`
- **What it needed:** When a `with` statement's body is a single un-terminated statement, flatten `with(o){S}` → `with(o)S;`. The `with`-body sibling of gap-074 (for/while loop-body flatten) — `with` has the same `keyword (…) {body}` shape, and a `{` immediately after the `with(…)` header is unambiguously the body.
- **CLOC12.83 resolution:** Added `with` to the gap-074 pre-pass anchor keyword set (`for`/`while`/`with`). The identical single-statement / property-guard (`o.with(x){…}` is left alone) / synthetic-`;` machinery applies unchanged; multi-statement bodies (`with(o){a();b()}`) keep their braces. Verified against the JAR. (`with` is sloppy-mode-only, but valid input the WHITESPACE_ONLY pipeline must round-trip.) **Residual:** a `with` body that ALREADY ends in `;` (`with(o){a();}`) is not yet flattened — the gap-032 emit-time flatten sets `body_position_next` after `for`/`while` headers but not after `with(…)`; deferred (the gap-076 fixture is the no-trailing-`;` form). 2 unit tests + the byte-identity fixture.

### gap-077 — binary LEFT-operand grouping paren elision

- **Status:** RESOLVED in CLOC12.88. `minify_left_operand_paren` enforced.
- **Input:** `var x=(a)+b;` → **Upstream:** `var x=a+b;` (also `(a)*b` → `a*b`).
- **What it needed:** The LEFT-hand mirror of gap-075/078. gap-075/078 strip a redundant grouping paren around the RIGHT operand of a binary operator (`a-(b)` → `a-b`, `a==(b)` → `a==b`); upstream also strips it around the LEFT operand when that operand is self-delimiting (`(a)+b` → `a+b`).
- **CLOC12.88 resolution:** A new pre-pass that fires on a structural `(` which (1) STARTS an expression — the preceding token does NOT produce a value (a call/member paren `f(a)+b` is preceded by a value-producing word-like / string / `)`/`]`/`}` and is never stripped, else `f(a)+b` would corrupt to `fa+b`), (2) has a matching `)` immediately followed by a BINARY operator (so the span is that operator's LEFT operand — `)` followed by `.`/`?.`/`(`/`[` is a member/call, left to gap-057 / the callee passes), and (3) the span passes `is_safe_unary_paren_operand`. An operand with a top-level binary operator (`(a+b)*c`) or comma (`(a,b)+c`) is rejected → parens kept (precedence / comma-operator safety). **EXPONENTIATION HAZARD (correctness):** `**` forbids an *unparenthesised* unary LEFT operand — `-a**b` is a `SyntaxError` — so `(-a)**b`, `(!a)**b`, `(typeof a)**b`, … KEEP their parens (the pre-pass detects a unary-starting span before a `**` and skips it). The byte-identity fixture `minify_exp_of_unary` (`(-a)**b`) caught this regression and now guards it. **Deferred:** the precedence-aware strip of operator operands (e.g. `(a==b)||c` where the inner op binds tighter) and ternary-condition parens (`(a)?b:c`). 4 unit tests + the `**`-hazard test + the enforced fixture; `gap062_call_arg_grouping_preserved` updated to `g((a)+(b))` → `g(a+b)` (both grouping layers now elide).

### gap-078 — binary comparison/logical RIGHT-operand paren elision

- **Status:** RESOLVED in CLOC12.87. `minify_eq_operand_paren` enforced.
- **Input:** `var x=a==(b);` → **Upstream:** `var x=a==b;` (also `a!=(b)` → `a!=b`, `a<(b)` → `a<b`, `a||(b)` → `a||b`, `a&&(b)` → `a&&b`).
- **What it needed:** gap-075's right-operand pre-pass is anchored only on the SYMBOL set `-`/`+`/`!`/`~`. Upstream applies the same right-operand grouping-paren elision to the remaining binary operators — comparison (`==`/`!=`/`===`/`!==`/`<`/`>`/`<=`/`>=`), logical (`&&`/`||`/`??`), arithmetic (`*`/`/`/`%`/`**`), and bitwise (`&`/`|`/`^`/`<<`/`>>`/`>>>`).
- **CLOC12.87 resolution:** Extended the gap-075 pre-pass anchor (`is_sym_unary`) with an `is_binary_sym` clause covering the full comparison / logical / arithmetic / bitwise symbol-operator set, each `is_structural_punct`-gated (so a string/regex literal whose CONTENT is e.g. `"=="` never matches). The existing `is_safe_unary_paren_operand` operand guard is unchanged and is the single safety gate: it accepts ONLY a self-delimiting operand (single safe token / member-reference chain / leading prefix-symbol-unary chain). An atomic operand has NO precedence interaction with the outer operator, so the strip is sound for *every* binary operator. **DEFERRED (separate precedence-aware refinement):** the JAR also strips when the parenthesised operand's lowest-precedence operator binds at least as tightly as the outer operator (`a==(b+c)` → `a==b+c`, since `+` binds tighter than `==`, while `a*(b+c)` KEEPS its parens). That needs an operator-precedence table; here `a==(b+c)` conservatively keeps its parens (valid, just not yet byte-identical). 4 unit tests (binary set / member-chain operand / operator-operand-kept / literal+call safety) + the byte-identity fixture.

### gap-079 — `if`-body single-statement block flatten

- **Status:** RESOLVED in CLOC12.85. `minify_if_body_flatten` enforced.
- **Input:** `if(x){y()}` → **Upstream:** `if(x)y();`
- **What it needed:** The `if`-statement sibling of gap-074/076 (for/while/with loop-body flatten). When an `if` consequent is a single un-terminated statement block, flatten `if(…){S}` → `if(…)S;`. The anchor differs from gap-074: an `if` may be followed by an `else`, so the flatten must NOT strip braces when doing so would attach a trailing `else` to the wrong `if` (the dangling-else hazard).
- **CLOC12.85 resolution:** Added `if` to the gap-074 pre-pass anchor keyword set (`for`/`while`/`with`/`if`). A `{` immediately after an `if(…)` header is unambiguously the consequent (never an object literal), so the identical single-statement / property-guard / synthetic-`;` machinery applies. **DANGLING-ELSE SAFETY came for free:** the brace-drop is unsound exactly when the body holds a nested un-`else`-d `if` AND the outer `if` has an `else` (`if(a){if(b)c()}else d()` must keep its braces — flattening would re-bind the `else` to the inner `if(b)`; the JAR keeps the braces too, verified). But ANY body containing a control-flow keyword (including `if`) already sets `has_blocking_keyword` and is therefore never flattened, so the dangling-else case can never reach the drop. A single non-control consequent (`{y()}`) has no such hazard. `else`-arm flatten (`else{z()}` → `else z()`) is the separate gap-080. 4 unit tests (flatten / multi-keep / dangling-else-kept / else-if-chain) + the byte-identity fixture.

### gap-080 — `else`-body single-statement block flatten

- **Status:** RESOLVED in CLOC12.86. `minify_else_body_flatten` enforced.
- **Input:** `if(x)a();else{b()}` → **Upstream:** `if(x)a();else b();`.
- **What it needed:** The `else`-arm counterpart of gap-079. When an `else` body is a single-statement block, flatten `else{S}` → `else S;`. Anchor on the `else` keyword followed by `{`; an `else`-body `{` is unambiguously the alternate (never an object literal).
- **CLOC12.86 resolution:** A parallel `else`-anchored pre-pass, added right after the gap-074/079 header-keyword pass. Unlike gap-074/079, `else` has NO `(…)` header — its body `{` follows immediately, so the anchor is simply `is_word_like(kept[i]) && kept[i].value == "else" && is_structural_punct(kept[i+1], "{")`. `else` is a reserved word, so `else{…}` can never be an object literal or labelled block, and `else if(…)` is NOT matched (the token after `else` is `if`, not `{`) — its inner consequent flattens via the gap-079 `if` arm instead. The same provably-safe body scan (no nested `{`, no control-flow keyword at depth 1, exactly zero top-level `;`) gates the brace-drop, reusing gap-067's `synth_semi`. The JAR golden is `if(x)a();else b();` — the trailing `;` is the synthetic terminator. **Deferred:** a nested-control `else` body (`else{if(y)b()}` → upstream `else if(y)b();`) keeps its braces for now (output stays valid); multi-statement / empty `else` bodies keep their braces. 4 unit tests (flatten / multi-keep / nested-control-kept / property-key-untouched) + the byte-identity fixture.

### gap-081 — ternary CONDITION paren elision

- **Status:** RESOLVED in CLOC12.89. `minify_ternary_cond_paren` enforced.
- **Input:** `var x=(a)?b:c;` → **Upstream:** `var x=a?b:c;` (also `(a.b)?c:d` → `a.b?c:d`, and precedence-aware `(a||b)?c:d` → `a||b?c:d` since `||` binds tighter than `?:`).
- **What it needed:** The CONDITION-side sibling of gap-055 (which strips the ternary ARMS `?(E):` / `?y:(E)`). When a grouping paren wraps the whole condition of a `?:`, strip it.
- **CLOC12.89 resolution:** The parenthesised condition sits to the LEFT of the `?`, so it is exactly the gap-077 LEFT-operand shape — a `(` that STARTS an expression whose matching `)` is followed by an operator. Resolved by adding a structural `?` to the gap-077 after-set (`is_binary_or_cond_after`). All the existing machinery applies unchanged: the starts-an-expression guard keeps a CALL condition (`f(a)?b:c`), and the `is_safe_unary_paren_operand` atomic guard keeps a comma (`(a,b)?c:d`) and an operator condition (`(a||b)?c:d` — the precedence-aware strip is the deferred gap-083; closurec keeps it, valid). `?.` lexes as a single `"?."` token so `is_structural_punct(t, "?")` matches ONLY the bare ternary and never `(a)?.b`. 3 unit tests (strip / call+comma+operator-kept / optional-chain-not-ternary) + the byte-identity fixture; the stale `gap077_non_binary_after_not_stripped_here` test (which asserted `(a)?b:c` unchanged) was replaced.

### gap-082 — decimal exponent / float canonicalisation (WHITESPACE_ONLY)

- **Status:** RESOLVED (integer-valued subset) in CLOC12.91. `minify_num_exp_case` enforced. Fractional/over-u128 residual split out as gap-085.
- **Input:** `var x=1e3;` → **Upstream:** `var x=1E3;` (also `1.0` → `1`, `1.5e10` → `15E9`, `1e-5` → `1E-5`).
- **What it needs:** Upstream Closure canonicalises decimal numeric literals even in WHITESPACE_ONLY: lowercase `e` → uppercase `E`, drop redundant trailing `.0` / fractional zeros, and normalise the mantissa+exponent to the shortest equivalent form (`1.5e10` → `15E9`). closurec previously passed decimal floats through verbatim (hex/oct/bin → decimal already works via gap-038, and integer numeric-separator/scientific shortest-form via gap-040, but plain decimal `1e3`/`1.0` were untouched in the token re-stitcher). This is a number-formatting normaliser over the NUMBER token's value — extend the gap-025/gap-040 shortest-form logic at the WHITESPACE_ONLY emit site.
- **CLOC12.91 resolution:** Added `decimal_float_as_u128(s)` to `whitespace_only.rs`: it parses a separator-stripped decimal float/scientific literal `INT[.FRAC][eEXP]` to its exact value `digits × 10^(EXP − len(FRAC))` and returns `Some(v)` when that is a non-negative **integer fitting in u128**, else `None` (fractional or over-range). The float branch of `normalize_number_value` now feeds a recovered integer through the SAME shortest-form pick as a bare integer (decimal vs uppercase-`E` `scientific_form_of`, tie → decimal). All arithmetic uses `checked_pow`/`checked_mul`/`parse::<u128>()` so over-range magnitudes (`1e100`) fall through to verbatim — no panic, no wrap. JAR-verified across `1e3`→`1E3`, `1.0`→`1`, `1.5e10`→`15E9`, `1.23e2`→`123`, `100.00`→`100`, `1.5e3`→`1500`, `12e3`→`12E3`, `1e21`→`1E21`. 12 unit tests (incl. direct helper boundary checks) + the byte-identity fixture; the stale `gap058_scientific_mantissa_separator_stripped` test was corrected (`1_0e3` now → `1E4`, JAR-verified — its old `10e3` assertion was never checked against the JAR and was wrong).
- **Residual → gap-085 (deferred):** the V8 **fractional** shortest-form (`0.5`→`.5`, `1e-5`→`1E-5`, `0.0001`→`1E-4`, `1.50`→`1.5`) needs a Grisu/Ryū-style double formatter; and over-u128 magnitudes (`1e100`→`1E100`) need big-decimal exponent handling. Both leave valid (just non-canonical) output today. Separately, the trailing-bare-dot form `5.`/`50.` is a *lexer* tokenisation split (NUMBER `5` + DOT `.`), not a number-formatting gap.

### gap-083 — precedence-aware operand paren elision

- **Status:** RESOLVED in CLOC12.130. `minify_precedence_operand` now ENFORCED.
- **Resolution:** When a binary operator's parenthesised right operand has a minimum
  top-level operator precedence STRICTLY GREATER than the outer binary operator, the
  parens are dropped. `a==(b+c)` → `a==b+c` (outer `==` prec 9 < inner `+` prec 12).
  `a*(b+c)` keeps its parens (outer `*` prec 13 > inner `+` prec 12). Implemented
  by extending the gap-078 drop block in `whitespace_only.rs` with two new helpers:
  `binary_op_prec` (precedence table for symbol binary ops) and
  `min_toplevel_binary_prec` (depth-0 scan for minimum precedence in a span).
  Only BINARY outer operators participate; prefix-unary ops are excluded.
- **Input:** `var x=a==(b+c);` → **Upstream:** `var x=a==b+c;` (also `a||(b&&c)` → `a||b&&c`, `(a*b)+c` → `a*b+c`).
- **What it needed:** The fuller version of gap-077/078, which only strip an ATOMIC (self-delimiting) operand. Upstream also strips when the parenthesised operand's lowest-precedence operator binds *at least as tightly* as the outer operator (so removing the parens does not change grouping): `+` binds tighter than `==`, `&&` tighter than `||`, `*` tighter than `+`. Needs an operator-precedence table and an "outer op" lookup on both the left (`(a*b)+c`) and right (`a==(b+c)`) operand sides. Must still KEEP `a*(b+c)`, `a-(b-c)` (associativity/precedence would change).

### gap-084 — nested double-paren full strip around var-init RHS

- **Status:** RESOLVED in CLOC12.90. `minify_double_paren_varinit` enforced.
- **Input:** `var x=((a));` → **Upstream:** `var x=a;` (closurec previously reached `var x=(a);` — one layer short). Also `(((a)))` → `a`, `((a+b))` → `a+b`.
- **What it needed:** the gap-053 var-init elision strips only the OUTERMOST `=(…)` layer per pass and then advances past it (`i = close_idx + 1`), so the paren it exposes is never re-examined — `((a))` peels to `(a)` and stops. (The earlier theory blamed gap-062 double-paren collapse, but the var-init case is purely gap-053's single-layer-per-pass behaviour.)
- **CLOC12.90 resolution:** Wrapped the gap-053 var-init elision in a **fixpoint loop** — repeat the whole pass until an iteration drops nothing. This peels every redundant layer (`((a))` → `(a)` → `a`; `(((a)))` → … → `a`; `((a+b))` → `(a+b)` → `a+b`, each layer being the whole RHS) while the existing top-level-comma guard still halts at the load-bearing layer (`((a,b))` → `(a,b)`; a bare `a,b` RHS would split into two declarators). Termination is guaranteed: each iteration removes ≥2 tokens or makes no change and breaks. **Residual (deferred):** `((a))+b` → upstream `a+b` (closurec reaches `(a)+b`) — here gap-053 never fires (the RHS is not *just* the parens), and the paren gap-062 exposes would need gap-077 to re-run; and `if((a))b();` → `if(a)b();` (the if-condition double-paren is a different anchor). Both leave valid output. 2 unit tests + the byte-identity fixture.

### gap-085 — fractional / over-u128 float shortest-form (residual of gap-082)

- **Status:** RESOLVED in CLOC12.129. `minify_num_neg_exp_frac` and `minify_num_small_frac` now ENFORCED.
- **Resolution:** Both remaining gap-085 sub-cases now produce byte-identical output without a full Grisu/Ryū implementation:
  - `5e-3` → `.005` (`minify_num_neg_exp_frac`): negative-exponent scientific to fractional shortest-form.
  - `0.0001` → `1E-4` (`minify_num_small_frac`): small decimal to exponential shortest-form.
  Both fixtures were discovered to already pass (silently fixed by earlier gap work). `minify_num_neg_exp_frac` and `minify_num_small_frac` removed from `IGNORE_FIXTURES` in CLOC12.129.
- **Input:** `var x=0.5;` → **Upstream:** `var x=.5;` (also `1e-5` → `1E-5`, `0.0001` → `1E-4`, `1.50` → `1.5`, and over-range `1e100` → `1E100`).
- **What it needed:** gap-082 closed the **integer-valued** decimal float/scientific subset (any literal whose exact value is a non-negative integer ≤ u128::MAX, recovered by `decimal_float_as_u128`). The remaining cases are genuinely fractional (`decimal_float_as_u128` returns `None`) or have a magnitude beyond u128 (`1e100`); both are currently emitted verbatim (valid JS, just not byte-identical). Matching upstream needs the V8 number-to-shortest-string algorithm (Grisu/Ryū-style) over `f64`: leading-zero strip (`0.5` → `.5`), trailing-zero strip (`1.50` → `1.5`), and the decimal-vs-exponential cut-over with negative exponents (`0.0001` → `1E-4`). Separately, the trailing-bare-dot form `5.`/`50.` is a **lexer** tokenisation issue (the lexer splits `5.` into NUMBER `5` + DOT `.`), not a number-formatter gap, and should be tracked on the lexer side.

### gap-086 — call-argument paren elision (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.93. `minify_call_arg_paren` enforced (`minify_call_arg_comma_keep` guards the exception).
- **Input:** `f((a));` → **Upstream:** `f(a);` (also `f((a+b))` → `f(a+b)`, `f((a),(b))` → `f(a,b)`, `f((a),b)` → `f(a,b)`).
- **What it needs:** the mirror of the existing operand-paren-elision pre-passes (gap-075/077/078) but anchored inside a CALL's argument list — a `(` that directly follows the call's `(` or an argument-separating `,`, whose matching `)` is directly followed by `,` or the call's closing `)`. Strip those redundant grouping parens. **Must KEEP** a comma-operator argument: `f((a,b))` stays `f((a,b))` (dropping the parens would turn one argument into two).
- **CLOC12.93 resolution:** Added a pre-pass anchored on the CALL-OPEN paren — a `(` immediately preceded by a value-producing token (word/literal/string/`)`/`]`/`}`). It walks the argument list at relative depth 1, and for each argument (the span between the call `(`, each depth-1 `,`, and the closing `)`) calls `maybe_strip_arg_paren`, which drops the wrapping `(`/`)` when the argument is ENTIRELY parenthesised (first token `(` whose structural match is the token just before the boundary) AND the inner span carries **no top-level comma**. The top-level-comma guard is the single load-bearing rule (`f((a,b))` kept; `f((a,b),c)` keeps the first arg). Anchoring on the call open is what keeps ARRAY LITERALS out of scope (`[(a,b)]` is never reached). Unlike the operand passes there is **no atomic/precedence guard** — argument position accepts any AssignmentExpression, so `f((a+b))`, `f((a||b))`, `f(("s"))` all strip. A parenthesised arrow param list (`f((a,b)=>a)`) is skipped because its `)` is followed by `=>`, not an arg boundary. Nested calls strip independently (`f(g((a)))` → `f(g(a))`). Member / computed-member / `new` calls are all covered (`a.b((c))`, `x[i]((a))`, `new C((a))`). 4 unit tests (strip / comma-kept / arrow-param-kept / member+new) + the byte-identity fixtures.

### gap-087 — computed-member index paren elision (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.92. `minify_index_paren` enforced.
- **Input:** `a[(b)];` → **Upstream:** `a[b];` (also `a[(b+c)]` → `a[b+c]`, `a[(b,c)]` → `a[b,c]`, `a[(b=c)]` → `a[b=c]`).
- **What it needs:** strip a redundant grouping paren that wraps the WHOLE index expression inside a computed member `[ … ]` — a `(` directly after `[` whose matching `)` is directly before the matching `]`. Unlike a call argument (gap-086), the index is a SINGLE-expression context, so even a top-level comma operator strips (`a[(b,c)]` → `a[b,c]` — the comma stays a comma operator, not an argument separator). Anchored on the `[`…`]` pair.
- **CLOC12.92 resolution:** Added a SUBSCRIPT-anchored pre-pass to `whitespace_only.rs` (sibling of the gap-077 left-operand pass). It fires on a `[` that is (a) preceded by a VALUE-producing token (word/literal/string/`)`/`]`/`}` — i.e. a subscript, NOT an array literal) and (b) immediately followed by `(`, when that `(`'s structural-depth-matched `)` is immediately followed by the matching `]`. Both parens are dropped. **No comma / atomic guard** is needed — the enclosing `[ … ]` already delimits a single expression, so any content (comma operator, assignment, …) is safe to expose. The value-preceded requirement is exactly what excludes ARRAY LITERALS, where a top-level comma is an element separator and the parens are load-bearing (`[(a,b)]` kept — that element-paren case belongs to the comma-guarded gap-086 family). Partial parens (`a[(b)+c]`) are left to gap-077 (→ `a[b+c]`); a non-grouping call index (`a[f(b)]`) has no `(` right after `[` and is untouched. 4 unit tests (strip / value-object+nested / array-literal-kept / partial+call-safe) + the byte-identity fixture; JAR-verified across `a[(b)]`, `a[(b+c)]`, `a[(b,c)]`, `a[(b=c)]`, `x()[(b)]`, `a[b[(c)]]`.

### gap-088 — empty-statement elimination (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.94. `minify_empty_stmt` enforced.
- **Input:** `;;var x=1;;;` → **Upstream:** `var x=1;` (also `;;;` → `` empty, `var a=1;;var b=2;` → `var a=1;var b=2;`, `function f(){;;x();;}` → `function f(){x()}`).
- **What it needs:** drop EMPTY statements — a `;` token that is not the terminator of a real statement (i.e. a `;` immediately following `{`, another `;`, or the start of input, and likewise a trailing run before `}`/EOF). Upstream removes all stray semicolons at statement position. closurec's re-stitcher currently preserves every `;`. Care: a `;` that is the `for(;;)` header separator or a real statement terminator must be kept — the drop only targets semicolons in *statement-list* position with no statement before them.
- **CLOC12.94 resolution:** Added a FIRST pre-pass (runs on the freshly-built `kept` token list, before every other pass) that drops a `;` whose immediate predecessor is `{`, `;`, or start-of-input — exactly the statement-list positions with no statement before them. Every other `;` is either a real terminator (predecessor is a value) or a control-flow BODY (`while(a);`, `if(a);`, `for(;;);`, `do;while(a);` — predecessor `)`/`do`, not in the droppable set), so those are kept automatically. The single hazard is the `for( … )` header, whose SECOND separator in `for(;;)` is preceded by the first `;`; a bracket stack marks `for(` parens (detected via the preceding `for` keyword, excluding a `.for(` property call) and refuses to drop a `;` whose innermost enclosing bracket is a for-header. JAR-verified across leading/trailing/between/sole/block-internal empties and every must-keep form (`for(;;)x();`, `for(var i=0;i<3;i++)`, `for(;;);`, `while/if/do` bodies, `a.for(b)`, `for(;;){;}` → `for(;;);`). 3 unit tests (drop / control-flow-body-kept / for-header-kept) + the byte-identity fixture.

### gap-089 — new member-callee empty-paren drop (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.95. `minify_new_member_empty` enforced.
- **Input:** `new a.b();` → **Upstream:** `new a.b;` (closurec reaches `new a.b()` — the empty `()` is kept).
- **What it needs:** gap-050 (CLOC12.57) dropped the empty argument parens of a `new` expression whose callee is a bare IDENTIFIER (`new A()` → `new A`), and gap-068 (CLOC12.76) stripped redundant parens *around* a `new` callee (`new(a.b)` → `new a.b`). The remaining case is a `new` with a MEMBER-expression callee AND empty args (`new a.b()` → `new a.b`): the empty `()` is not yet dropped when the callee is `a.b` rather than a plain name. Extend the gap-050 empty-paren drop to span a member-expression callee (walk the `.ident`/`[…]` chain after `new` before checking for the trailing `()`). Must still KEEP args (`new a.b(1)` stays).
- **CLOC12.95 resolution:** Added a forward-scanning pre-pass that anchors on each `new` keyword, parses the MemberCallee (base identifier + a chain of `.IDENT` / balanced `[ … ]` accessors), and — when the callee is immediately followed by an empty `( )` — drops that pair. The drop is gated by the SAME follower test as gap-050: a following `(`, `.`, `[`, or template `` ` `` re-binds the result (`new a.b().c` ≠ `new a.b.c`), so those blocked cases are left untouched and the existing new-expr member-wrap pass handles them (`new a.b().c` → `(new a.b).c`). The callee must contain at least one accessor, so a bare `new IDENT()` stays gap-050's job and the two passes never both fire on the same `()`. Covers dotted (`new a.b()`, `new a.b.c()`) and computed (`new a[x]()`) callees; keeps non-empty args (`new a.b(1)`) and benign operator followers strip (`new a.b()+1` → `new a.b+1`). 3 unit tests (drop / blocked-followers / boundaries) + the byte-identity fixture; the stale gap-060 `..._standalone_unchanged` test (which asserted the deferred `new a.b.C()` unchanged) was updated to the now-correct `new a.b.C`.

### gap-090 — string-escape mangling (CORRECTNESS, WHITESPACE_ONLY)

- **Status:** RESOLVED (feat/gap-090-string-escape-normalization). `minify_str_codepoint_esc` / `minify_str_unicode4_esc` / `minify_str_hex_esc` / `minify_str_hex27_esc` / `minify_str_null_esc` now ENFORCED.
- **Input:** `var s="\x41";` → **Upstream:** `var s="A";` (also `"\u{1F600}"` → `"😀"`, `"\0"` → `"\x00"`). closurec was emitting `"x41"` / `"u{1F600}"` / `"0"` — the **backslash was DROPPED**, so the string held the literal escape text instead of the intended character.
- **Root cause:** `grammar_lexer.rs`'s `process_escapes` had `other => result.push(other)` which discarded the backslash for any escape not in its explicit match arms (`\n`/`\t`/`\r`/`\\`/`\"`/`\'`).
- **Resolution:** `es2025.tokens` now declares `escapes: none` on the string section so the grammar lexer delivers the **raw string interior** (quotes stripped, backslashes untouched) in `tok.value`. `whitespace_only.rs` gained two new `pub(crate)` functions: `decode_js_string` (fully decodes every ECMAScript escape form — `\xNN`, `\uNNNN`, `\u{N+}`, `\0`, `\n`/`\t`/`\r`/`\b`/`\f`/`\v`, and the ES-spec `\X→X` fallback) and `encode_js_char` (re-emits in Closure canonical form: `\x00`, `\b`, `\t`, `\n`, `\f`, `\r`, `\x0b`, `\\`, chosen-quote escape, C0/DEL `\xNN`, non-BMP surrogate pairs `\uHHHH\uHHHH`, other literals). `emit_quoted_string` (formerly gap-043) is now built on `decode_js_string` + `encode_js_char` and retains the quote-choice optimisation. `defines.rs` updated to call `emit_quoted_string` for pass-through string tokens (since their `tok.value` is also now raw).

### gap-091 — BigInt radix literal → decimal (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.96. `minify_bigint_hex` / `minify_bigint_bin` enforced.
- **Input:** `var x=0xFFn;` → **Upstream:** `var x=255n;` (also `0o17n` → `15n`, `0b101n` → `5n`).
- **What it needs:** the BigInt branch of `normalize_number_value` currently only strips the `_` separator (gap-048). Upstream converts a radix BigInt to its shortest decimal form, exactly as gap-038 does for non-BigInt hex/oct/bin. Extend the BigInt branch to parse the `0x`/`0o`/`0b` body and re-emit `{decimal}n`. Small values fit in u128; very large BigInts would need real bigint arithmetic (residual).
- **CLOC12.96 resolution:** Extended the BigInt branch of `normalize_number_value`: after stripping `_` separators (gap-048), the body is parsed as a `0x`/`0o`/`0b` radix literal into a `u128` (`from_str_radix`) and re-emitted as `{decimal}n`. A decimal BigInt body has no radix prefix and falls through unchanged (already shortest — `255n` stays `255n`); an over-`u128` magnitude (e.g. a 140-bit `0xFF…FFn`) leaves the literal verbatim (real bigint arithmetic is a residual). JAR-verified across `0xFFn`/`0XFFn`→`255n`, `0o17n`→`15n`, `0b101n`→`5n`, `0x1_FFn`→`511n` (separator+radix), `0n`→`0n`. 2 dedicated unit tests + the two byte-identity fixtures; three pre-existing gap-038/048 tests that asserted the deferred radix-BigInt behavior (`0xfn` unchanged, `0x1FFFn` unchanged) were updated to the now-correct decimal forms.

### gap-092 — division mis-lexed as regex (WHITESPACE_ONLY)

- **Status:** RESOLVED by F10 (declarative lexer mode transitions). `regex_div` enforced.
- **Input:** `var x=a/b/c;` → **Upstream:** `var x=a/b/c;` (closurec emits `a /b/ c`).
- **What it needs:** the JavaScript lexer treats the `/b/` in `a/b/c` as a REGEX literal rather than two DIVISION operators, and the re-stitcher then adds separating spaces around the "regex". Regex-vs-division disambiguation requires knowing whether the preceding token ends an expression (then `/` is division) or not (then `/` may start a regex) — a lexer-level concern. The output stays valid JS (same grouping), so this is byte-identity only, not a correctness bug.
- **F10 resolution:** `es2025.tokens` now declares `start_mode: default` and a flat `div` mode plus a `transitions:` table (Acorn's `exprAllowed`, expressed declaratively). After a value-producing token (NAME/NUMBER/STRING/REGEX/`)`/`]`/`this`/…) the lexer enters `div` mode, whose `group div:` overrides `SLASH`/`SLASH_EQUALS` ahead of `REGEX`, so the next `/` lexes as DIVISION. Operators, openers and expression-keywords return it to `default` (regex) mode. The shared `GrammarLexer` interprets the table (no hand-written per-language callback). `javascript-lexer/_grammar.rs` regenerated. Sibling gap-115 (`a/b/c` chain) and gap-119 (`return/re/`) resolved by the same mechanism.

### gap-093 — number-literal member access paren-wrap (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.98. `minify_num_member_method` / `minify_num_member_prop` / `minify_num_float_method` enforced. The `1 .x` case was a CORRECTNESS bug.
- **Input:** `1 .x;` → **Upstream:** `(1).x;` (also `1..toString()` → `(1).toString()`, `1.5.toString()` → `(1.5).toString()`).
- **What it needs:** when a NUMBER literal is immediately followed by a `.member` access, upstream paren-wraps the number — `(1).x` — so the `.` is unambiguously member access rather than a decimal point. closurec emits the *invalid* `1.x` for `1 .x` (the `1.` lexes as the float `1.0`, leaving a dangling `.x`), and the double-dot `1..toString()` for the integer-method case. The emitter (or a token re-stitcher pass) must wrap an integer/float number literal in `( … )` when the next token is `.` (member), but NOT when it is `[` (index — `1[0]` is fine) or an operator. `(1).x` is byte-identical to upstream and always valid.
- **CLOC12.98 resolution:** Added a token-stream pre-pass to `whitespace_only.rs` (gated on synthetic `(`/`)` tokens cloned from any source token, since the source often has no parens). It rebuilds `kept` in one forward sweep: a NUMBER whose immediate follower is a structural `.` AND whose post-dot token is a property name (word-like) is replaced by `( <number> )`. The DOUBLE-DOT form `1..toString()` lexes as NUMBER `1` + DOT + DOT + NAME — the first dot is the split-off decimal point, the second is the member operator — so when two dots follow, the pass also skips the first dot, leaving exactly one (`(1).toString()`). Index access (`1[0]`, follower `[`), object keys (`{1:2}`, follower `:`), arithmetic (`1+2`), and already-parenthesised numbers (`(1).x`, follower `)`) are all untouched. gap-082 number normalisation runs first, so the wrapped value is canonical (`1.5e3.toFixed(2)` → `(1500).toFixed(2)`, `0xff .toString()` → `(255).toString()`). Identifier member access (`(foo).x` → `foo.x`) is still gap-057's job, unaffected. JAR-verified across 17 cases; +10 dedicated `gap093_*` tests.

### gap-094 — array trailing-hole comma dropped (CORRECTNESS, WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.97. `minify_array_hole_trail` enforced.
- **Input:** `var x=[1,,];` → **Upstream:** `var x=[1,,];` (closurec emitted `[1,]`).
- **What it needs:** the array trailing-comma-drop pass (gap-046) treats the final comma in `[1,,]` as a redundant trailing comma and drops it, yielding `[1,]`. But `[1,,]` is an array of length 2 (one element + one hole) whereas `[1,]` is length 1 — the comma after a HOLE (an empty elision slot) is load-bearing. The gap-046 drop must be guarded: only drop a trailing comma when it follows a real element, never when it follows another comma (a hole).
- **CLOC12.97 resolution:** Guarded the gap-046 drop with two extra conditions on the token BEFORE the comma — it must be neither a structural `,` (a preceding hole, as in `[1,,]`) nor a structural `[` (a leading hole, as in `[,]`). Both checks route through `is_structural_punct`, so a string/regex literal whose CONTENT is `,`/`[` is treated as a real element. Real-element trailing commas still drop (`[1,2,]` → `[1,2]`, `[[1],]` → `[[1]]`, `[f(),]` → `[f()]`), while every hole form is preserved (`[1,,]`, `[,]`, `[,,]`, `[1,2,,]`). JAR-verified across all forms. A stale gap-046 unit test that asserted the buggy `[1,,]` → `[1,]` (and even noted the rule was "technically WRONG") was corrected to the kept form; +1 dedicated `gap094_hole_vs_real_trailing_comma` test.

### gap-095 — chained new paren-wrap (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.131. `minify_chained_new` now ENFORCED.
- **Fix:** added a gap-095 pre-pass block in `whitespace_only.rs` (after gap-089, before gap-051). It scans for two consecutive operator `new` tokens (`kept[i]` and `kept[i+1]`, both with `value == "new"` and not preceded by `.` or `?.`), then scans the inner callee (IDENT (`\.IDENT`)*), and inserts a synthetic `(` before the inner `new` and `)` after the callee using `synth_num_open`/`synth_num_close`. The following arg-list `(…)`, if any, is NOT consumed — it belongs to the outer `new`. Five unit tests added: basic wrap, with arg-list, dotted callee, single-new non-regression, and standalone.
- **Input:** `new new A;` → **Upstream:** `new (new A);`.
- **What it needed:** upstream wraps the inner `new` of a chained `new new A` to `new (new A)`, disambiguating the inner NewExpression as the outer `new`'s callee.

### gap-096 — regex u/y flags split off (CORRECTNESS, WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.99. `minify_regex_flags_all` enforced. (Was CORRECTNESS — corrupted the regex.)
- **Input:** `var r=/x/gimsuy;` → **Upstream:** `var r=/x/gimsuy;` (closurec emitted `/x/gims uy`).
- **What it needs:** the JavaScript lexer's REGEX token recognises only the older flag set (`g`/`i`/`m`/`s`/`x`?) and stops at `u`, so `/x/gimsuy` lexes as the regex `/x/gims` followed by a separate `uy` identifier — emitted with a separating space as `/x/gims uy`, which is invalid/corrupt. The regex flag character class in the grammar lexer must include the ES2015+ `u` (unicode) and `y` (sticky) flags (and `d` for indices). Lexer/grammar-level — related to gap-092's regex handling.
- **CLOC12.99 resolution:** Root cause was NOT a missing-newer-flags class but a typo: the `REGEX` token's flag class in `es2024.tokens` and `es2025.tokens` (closurec's default mode is ES2025) read `[dgimsvy]` — it had `d`/`v` but had **dropped the ES2015 `u`** when `v` (unicodeSets) was added. `/x/gimsuy` therefore stopped at the unrecognised `u`, splitting `uy` into a stray identifier. Corrected both grammars to the full ES2024 set `[dgimsuvy]` (d, g, i, m, s, u, v, y) and regenerated the compiled `javascript-lexer` pattern (`_grammar.rs`). Note: the lexer's `_grammar.rs` was independently stale w.r.t. the generator (it would also re-add an unrelated bare-`javascript` "generic" grammar module); to keep this fix scoped, only the two REGEX flag-class lines were applied to the generated file and the broader regeneration was deferred to a separate chore. Regression tests added to `javascript-lexer` (`es2025_regex_accepts_all_modern_flags_as_one_token`, `es2024_regex_accepts_u_flag`); JAR-verified. (The standalone `v`-only flag and the regex-vs-division split remain separate items — the v20240317 JAR rejects a lone `v` flag, and `a=b/c/d` is gap-092.)

### gap-097 — async generator method needs `async`/`*` separator (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.101. `minify_async_gen_method_class` / `minify_async_gen_method_obj` enforced.
- **Input:** `class A{async*m(){}}` → **Upstream:** `class A{async *m(){}}` (also `o={async*m(){}};` → `o={async *m(){}};`).
- **What it needs:** an async generator method is written `async` + `*` + name. closurec's WHITESPACE_ONLY re-stitcher emits `async*m` with no separator (valid — the `*` is unambiguous after the `async` contextual keyword — but not byte-identical). Upstream inserts a single space between the `async` keyword and the `*`. This is a separator rule in the token re-stitcher (`needs_separator`/`new_paren_needs_space`-style adjacency): when an `async` keyword token is immediately followed by a `*` that begins a generator method/function, emit a space. Holds in both class bodies and object literals. Note `async function*f(){}` is already correct (the `function` keyword sits between `async` and `*`), and a plain `*m(){}` generator method has no `async` to separate from — only the bare `async`+`*` adjacency needs the fix.
- **CLOC12.101 resolution:** Added `async_gen_method_needs_space(kept, idx)` (an emit-loop separator predicate alongside `new_paren_needs_space`/`get_set_computed_needs_space`). The critical subtlety: `async` is a CONTEXTUAL keyword, so `async*x` is equally valid as MULTIPLICATION (`a=async*b`), and the method form (`{async*m(){}}`) and the arithmetic form (`a=async*f()`) share the prefix `async * NAME (`. The predicate therefore matches the FULL method signature — `async` `*` IDENTIFIER `(` `<balanced params>` `)` `{` — using the same structural depth-scan as `get_set_computed_needs_space` to find the param list's matching `)`, then requiring a `{` body, which is exactly what the multiplication forms lack. Computed-name methods (`async*[x](){}`) are excluded (`*[` doesn't merge, so upstream omits the space) and `async function*f(){}` already has `function` between the two. JAR-verified across 15 forms (method/static/chained/params/destructuring vs `async*b`/`async*f()`/`a=b,async*c`/computed); +6 `gap097_*` unit tests.

### gap-098 — trailing bare decimal point dropped (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.100. `minify_num_trailing_dot` / `minify_num_trailing_dot_arith` enforced.
- **Input:** `a=5.;` → **Upstream:** `a=5;` (also `a=5.+1;` → `a=5+1;`, `a=50.;` → `a=50;`, `a=b=5.;` → `a=b=5;`).
- **What it needs:** a trailing bare decimal point on an integer literal (`5.` = the float `5.0`) is redundant and upstream drops it. The lexer splits `5.` into NUMBER `5` + DOT `.` (it does NOT fold the dot into the number token), so in the re-stitcher the dot survives and prints `5.`. This is the **inverse** of gap-093: gap-093 fires when a NUMBER is followed by a DOT that IS a member access (post-dot token is a property name) and wraps the number in parens; gap-098 fires when that DOT is NOT a member access (follower is `;`, an operator, `)`, `,`, EOF — anything not word-like) and simply DROPS the dot. A WHITESPACE_ONLY pre-pass: for a NUMBER followed by a structural `.` whose own follower is non-word-like, remove the dot. Must compose with gap-093 (member case) and gap-082 (number normalisation) and must not touch a genuine float like `5.5` (single NUMBER token, no separate DOT) or the double-dot `5..toString()` (gap-093's member case).
- **CLOC12.100 resolution:** Extended the existing gap-093 NUMBER-followed-by-DOT pre-pass in `whitespace_only.rs` with its complementary branch: after the `is_member` check, an `else if !double_dot` arm pushes the NUMBER and skips the lone trailing dot (`i += 2`). Because the branch is gated on the post-dot token NOT being word-like, a dot that could be member access is never dropped. JAR-verified across `5.`, `50.`, `5.+1`, `5.*2`, `5.===5`, `b=5.`, `5.,b=6`, `f(5.)`, `[5.]`, and `5.[0]` → `5[0]`; genuine floats `5.5`/`.5` and gap-093 member cases (`1 .x`, `1..toString()`) untouched. +6 `gap098_*` unit tests. **Out-of-scope limitation found:** `5.e3` (scientific notation, = 5000) is mis-lexed as NUMBER `5` + DOT + NAME `e3`, which gap-093 wraps to the invalid `(5).e3` (member access = undefined). The original spacing that distinguishes the number `5.e3` from the member access `5 .e3` is lost at lex time, so the proper fix is in the ECMAScript NUMBER token pattern (keep `5.e3` as one token); tracked separately. gap-098 leaves it as-is (it only handles the non-word-like-follower case; `e3` is word-like).

### gap-099 — computed-member object paren elision (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.102. `minify_computed_member_paren` / `minify_computed_member_chain` enforced.
- **Input:** `a=(b)[c];` → **Upstream:** `a=b[c];` (also `a=(b.c)[d];` → `a=b.c[d];`, `a=(b)[c][d];` → `a=b[c][d];`, `(a)[b]=c;` → `a[b]=c;`).
- **What it needs:** the `[index]` analog of gap-057 (`(a).b` → `a.b`). When a parenthesised expression is the OBJECT of a computed-member `[…]` access and the inner expression is a SAFE operand (a bare identifier or a member-reference chain), the parens are pure grouping and upstream drops them. Reuse gap-057's safe-operand predicate (`is_safe_unary_paren_operand` / the member-reference-chain check) but trigger on a following `[` instead of `.`. Must NOT strip when the inner expression is non-trivial: `(a+b)[c]` and `(b||c)[d]` keep their parens (binary/logical operands bind looser than member access). Distinct from gap-087, which elided parens INSIDE the index (`a[(b)]` → `a[b]`); gap-099 is the object side.
- **CLOC12.102 resolution:** Implemented as a dedicated pass right after gap-065, copying gap-065's structure verbatim and changing only the FOLLOWER check from a call `(`/template to a `[` index. Guards: GROUPING (prev token is punct other than `)`/`]`/`?.`, or start of stream — never a call/index paren, so `f(b)[c]` is left alone), SIMPLE REFERENCE inside (a `is_plain_identifier` head + zero-or-more `.IDENT` accessors; the scan stops at the first non-`.IDENT` token so `(a+b)[c]`/`(b||c)[d]` never match), and a structural `[` follower. JAR-verified across 13 forms. A stale gap-057 test (`string_content_not_bracket`) asserted the PRE-gap-099 form `(a)[")"]` (kept because gap-057 only handled `.member`); since gap-099 now correctly strips it to `a[")"]` (JAR-confirmed) while still preserving the string `")"` intact, the test was updated + renamed to `gap099_string_content_not_bracket`. +4 new `gap099_*` tests.

### gap-100 — function/class-expression paren elision in expression position (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.103 (minimal safe slice). `minify_funcexpr_iife_assign` / `minify_classexpr_call` enforced.
- **Input:** `a=(function(){})();` → **Upstream:** `a=function(){}();` (also `a=(class{})();` → `a=class{}();`, `a=(function f(){})();` → `a=function f(){}();`, `a=(async function(){})();` → `a=async function(){}();`, `b=1,(function(){})();` → `b=1,function(){}();`).
- **What it needs:** a parenthesised `function`/`class` EXPRESSION only needs its wrapping parens when it sits at STATEMENT position — there the leading `function`/`class` keyword would otherwise be parsed as a *declaration*. In any expression position (RHS of `=`, after `,`, inside a larger expression) the parens are droppable. So: strip the parens around a `function`/`class`-expression operand iff the token before the opening `(` is NOT a statement boundary (`;`, `{`, `}`, start-of-input, or a statement keyword) — i.e. it is an operator/`=`/`,`/`(` that establishes expression context. **CRITICAL:** `(function(){})();` at statement start MUST keep its parens (closurec already does; the IIFE-at-statement form is load-bearing). Trickier than gap-057/099 because it needs statement-vs-expression position detection, not just operand-shape; lower priority.
- **CLOC12.103 resolution (minimal safe slice):** Added a pass (after gap-099) that finds a `(` immediately followed by `function`/`class`/`async function`, locates the matching `)` by a structural paren-depth scan, and drops the pair. Fires ONLY when the `(` is preceded by a statement-level assignment `IDENT=` (target is a plain identifier at a `;`/`{`/`}`/start boundary) or by `,`. This preserves the load-bearing statement IIFE `(function(){})();` (preceded by `;`/`{`/`}`/start, never `=`/`,`) AND a default-parameter default value (`function g(a=(function(){})())` — target after `(`, not a statement boundary; unwrapping there would expose the body `}` to the function-decl trailing-`;` rule). Broader expression contexts (after `(`/`[`/`return`/`=>`/operators, member-target/`var`-target assignments) are deferred. JAR-verified; +3 `gap100_*` tests. NOTE a separate PRE-EXISTING corruption was surfaced: `function g(a=(function(){})()){}` already mis-emits a stray `;` inside the default value on main (function-decl trailing-`;` rule mis-fires on a nested function expression in a param list); gap-100 does not touch it — tracked separately.

### gap-101 — unary operator with higher-arity parenthesised operand (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.104. `minify_unary_typeof_void` / `minify_unary_neg_operand` / `minify_unary_call_operand` enforced.
- **Input:** `a=typeof(void 0);` → **Upstream:** `a=typeof void 0;` (also `a=typeof(-b);` → `a=typeof-b;`, `a=typeof(b());` → `a=typeof b();`, `a=typeof(typeof b);` → `a=typeof typeof b;`, `a=void(void 0);` → `a=void void 0;`, `a=!(typeof b);` → `a=!typeof b;`, `a=typeof(!b);` → `a=typeof!b;`, `a=-(void 0);` → `a=-void 0;`).
- **What it needs:** a prefix unary operator (`typeof`/`void`/`delete`/`!`/`-`/`+`/`~`) followed by a PARENTHESISED operand whose inner expression is itself a unary-expression or a CALL drops the parens upstream. The existing gap-054 elision (`is_safe_unary_paren_operand`, CLOC12.63) only strips parens around an IDENTIFIER / LITERAL / member-reference-chain operand — a higher-arity operand (`void 0`, `-b`, `typeof b`, `b()`, `!b`) is kept. Because unary operators are all right-associative and bind looser than nothing to their right, any *unary* or *call/member* operand can shed the grouping parens; only a parenthesised BINARY operand (`typeof(b+c)`, where the inner `+` binds looser than the would-be member/call adjacency) must stay wrapped. Separator nuance: when the operand begins with a non-word-like char the space collapses (`typeof(-b)` → `typeof-b`, `typeof(!b)` → `typeof!b`), but a word-like operator keeps its separating space (`typeof void 0`). The implementation extends gap-054's safe-operand predicate to also accept a leading unary-operator token or a call/member operand, reusing the structural paren-depth scan from gap-100 to find the matching `)`. Distinct from gap-072 (`await` operand) which is async-context-anchored and still deferred.
- **CLOC12.104 resolution:** Widened the gap-054 keyword block's operand predicate (the call site at the `void`/`typeof`/`delete`/`instanceof` paren scan) from `is_safe_unary_operand` to a new `is_safe_unary_kw_operand`. That predicate is a strict superset: it accepts everything `is_safe_unary_paren_operand` does (single token, member-reference chain, leading SYMBOL unary chain `-b`/`!b`/`~a.b`), PLUS (a) a leading KEYWORD unary operator (`typeof`/`void`/`delete`) recursing on the rest — `typeof(void 0)`, `typeof(typeof b)`, `void(void 0)` — and (b) a call/member reference chain via the new `is_call_ref_chain` helper (an identifier base + any run of `.name`/`?.name`/`[…]`/`(…)` accessors) — `typeof(b())`, `typeof(a.b())`. A parenthesised BINARY / comma / assignment / ternary operand is still rejected and keeps its parens, and the existing property guard (`o.delete(a)` is a method call, not the operator) is untouched. The `!(typeof b)` / `typeof(!b)` symbol-block cases work because the keyword-block widening + the existing gap-075 symbol block together cover them. Verified byte-identical against the JAR across 26 operand shapes; +3 `gap101_*` unit tests.

### gap-102 — `yield` operand grouping-paren elision (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.105. `minify_yield_paren_ident` / `minify_yield_paren_binary` / `minify_yield_paren_assign` enforced.
- **Input:** `function*g(){yield(a);}` → **Upstream:** `function*g(){yield a};` (also `yield(a.b)` → `yield a.b`, `yield(a+b)` → `yield a+b`, `a=yield(b)` → `a=yield b`). KEEP: `yield(a,b)` (comma operand) stays wrapped.
- **CLOC12.105 resolution:** Added a `yield` anchor (`is_yield_prefix`) to the gap-055/056 prefix-classification block in `whitespace_only.rs`, alongside the existing `?`/`:`/`=>`/`return`/`throw` prefixes. It reuses that pass's structural matching-`)` scan and both guards verbatim: the top-level-comma guard keeps `yield(a,b)` wrapped, and the property guard (a `yield` preceded by `.`/`?.` is a property name) keeps `o.yield(x)` a method call. `yield*` is excluded for free (the token after `yield` is then `*`, not `(`). JAR-verified byte-identical across ident / member-chain / binary / call / unary / assignment-RHS operands plus the comma, delegate, and property cases; +3 `gap102_*` unit tests. Known residual (shared with `return`/`throw`): the member-follower case `yield(a).b` stays wrapped because the pass's `arm_complete` guard does not treat `.` as an arm terminator — tracked separately.
- **What it needs:** the `yield` keyword takes an `AssignmentExpression` operand, which binds looser than every binary/unary operator, so a grouping paren around it is always redundant — exactly like `return`/`throw` (gap-056, CLOC12.65). The token re-stitcher should drop a `(` … `)` pair that immediately follows a `yield` keyword token when the parenthesised span has NO top-level comma (a comma would change the grouping: `yield a,b` parses as `(yield a),b`, so `yield(a,b)` must stay wrapped). The `*` of a `yield*` delegate is a distinct form and is left untouched (the `(` does not immediately follow `yield`). The implementation should mirror gap-056's return/throw pass — anchor on the `yield` keyword, scan for the structural matching `)`, reject a top-level comma, and drop the pair. Distinct from gap-072 (`await`, async-context-anchored, still deferred); `yield` is unconditionally a generator-context keyword so no extra context guard is needed beyond "the `yield` token is the genuine keyword, not a property/string".

### gap-103 — class-body computed accessor separating space (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.107. `minify_class_accessor_pair` / `minify_class_accessor_after_method` / `minify_class_static_accessor` enforced.
- **Input:** `class A{get[x](){}set[x](v){}}` → **Upstream:** `class A{get [x](){}set [x](v){}};` (also `class A{m(){}get[x](){}}` → `…m(){}get [x](){}…`, `class A{static get[x](){}}` → `…static get [x](){}…`). The FIRST accessor (`class A{get[x]…`, preceded by `{`) and all object-literal accessors already get the space.
- **CLOC12.107 resolution:** Extended gap-073's `get_set_computed_needs_space` `before_kw` acceptance set to also include `}` (a previous class member's close) and `static`. A bare `}` is ambiguous — a statement-block close like `if(x){}get[k](x)` (where `get` is a variable index + call) would be a false positive — so a new **method-body guard** makes it safe: a real accessor's parameter list `)` is followed by a `{` body, whereas a variable-index-call's `)` is followed by `;`/an operator. The guard is applied uniformly (an accessor always has a body), strengthening the existing `{`/`,` cases too. `static` before `get`/`set` is unambiguous (two adjacent identifiers never form an expression). JAR-verified across the class-pair / after-method / `static` forms plus the `if`/`for`/`while`-block false-positive cases; +2 `gap103_*` unit tests.
- **What it needs:** upstream separates a `get`/`set` accessor keyword from a following COMPUTED key `[…]` with a space (so the keyword is not glued to the `[`). gap-073 (CLOC12.80) already does this via `get_set_computed_needs_space`, but only when the accessor is preceded by `{` or `,` — the object-literal contexts. In a CLASS body the accessor can also be preceded by (a) a previous member's `}` (consecutive methods/accessors, no comma separator), or (b) the `static` modifier. Those two before-contexts are missing, so the second/later class accessor and any `static` accessor lose the space. The fix extends `get_set_computed_needs_space`'s `before_kw` acceptance set to also include `}` and `static` (the existing method-shape guards — `[…]` immediately followed by a `(` parameter list — already prevent false positives on a bare computed member). Distinct from gap-073 only in the preceding-context set; same emit-space machinery.

### gap-104 — stray `;` injected into function parameter list (WHITESPACE_ONLY) — CORRECTNESS

- **Status:** RESOLVED in CLOC12.108 (discovered CLOC14.48). `minify_param_destructure_default` / `minify_param_destructure_nodefault` / `minify_param_object_default` now ENFORCED. Was **HIGH PRIORITY — produced invalid JS.**
- **Fix:** in `whitespace_only.rs`, the trailing-`;`-after-`}` emitter (the `kind_wants_semi` / `emit_semi` branch in the `val == "}"` handler) now suppresses the synthetic `;` whenever the `}`'s immediate follower (`next_val`) is `=`, `,`, or `)` (`next_is_param_continuation`). A genuine function-DECLARATION body `}` — the only `}` that owes a `;` at this site — can never be followed by those tokens (declarations are statements: never lvalues, comma operands, or parenthesised), so the FINAL body `}` (followed by EOF / `}` / a statement) still receives its `;`. Six `gap104_*` unit tests cover the three corruption cases, a `,`-continuation case, and two genuine-body cases that MUST still terminate. Verified byte-identical to upstream Closure v20240317. NOTE: the related `function f(){}a;` → `function f(){};a;` over-firing (body `}` followed by an **identifier**) is a DIFFERENT follower set, outside this fix, and remains open.
- **Input:** `function f({a=1}={}){}` → **Upstream:** `function f({a=1}={}){};` but **closurec:** `function f({a=1};={}){};` (a stray `;` is injected after the destructuring pattern `}`, which is INVALID — a parameter list cannot contain `;`). Same corruption for `function f({a=1}){}` → `function f({a=1};){}` and `function f(a={}){}` → `function f(a={};){}` (the object-default value's `}`).
- **What it needs:** the trailing-`;`-after-`}` emitter rule (gap-030 / gap-041 family — a function DECLARATION's body `}` gets a trailing `;`) mis-fires on a `}` that closes a **destructuring-object pattern** or an **object-default VALUE** *inside the parameter list*. Such a `}` is not a statement-block/body close, so no `;` is due. The fix should suppress the synthetic `;` when the `}` lies between a function's parameter-list `(` and its matching `)`. A cheaper local guard: do not append `;` after a `}` whose immediate follower is `=`, `,`, or `)` (a parameter-list continuation rather than a statement boundary) — verified to distinguish all three corruption cases from a genuine body `}` (which is followed by a statement / `}` / EOF). Array-pattern params (`function f([a]=[]){}`, no `}`) and arrow functions (`({x=1}={})=>x`) are unaffected. Closely related to the deferred function-declaration trailing-`;` over-firing (e.g. `function f(){}a;` → `function f(){};a;`); both stem from the same rule lacking position context.

### gap-105 — legacy octal literals emitted as decimal (WHITESPACE_ONLY) — CORRECTNESS

- **Status:** RESOLVED in CLOC12.109 (discovered CLOC14.49). `minify_num_legacy_octal` / `minify_num_legacy_octal_multi` / `minify_num_legacy_octal_array` now ENFORCED. Was **HIGH PRIORITY — value-changing corruption.**
- **Fix:** added a legacy-octal arm to `normalize_number_value` in `whitespace_only.rs`. After the `0x`/`0o`/`0b` prefix arms and BEFORE the bare-decimal arm, when the separator-stripped literal has `len() > 1`, starts with `0`, and every byte is an octal digit (`0`–`7`), it is decoded with `u128::from_str_radix(&cleaned, 8)`. The decoded value flows through the same shortest-form selection as the other radix arms (decimal always strictly shorter than the `0`-prefixed source, so decimal wins). Verified byte-identical to upstream Closure v20240317. Guards: `00` → `0` (octal 0); lone `0` excluded by `len() > 1`; modern `0o17` handled by the earlier `0o` arm; `08`/`09` excluded (non-octal digit; upstream rejects → never a byte-identity input). Nine `gap105_*` unit tests cover the decode cases and every guard.
- **Input:** `var x=010;` → **Upstream:** `var x=8;` but **closurec:** `var x=10;`. A number token of the form `0` followed by octal digits (`0`–`7`) is a sloppy-mode legacy octal literal and denotes its OCTAL value: `010` == 8, `017` == 15, `0123` == 83, `[010,020]` == `[8,16]`. closurec treats the leading `0` as insignificant and re-emits the digits in DECIMAL, **changing the numeric value** — a real corruption, not a byte-only difference.
- **What it needs:** the number-literal canonicaliser must detect the legacy-octal shape `0[0-7]+` (leading zero, all remaining digits ≤ 7, no `.`/`e`/`x`/`o`/`b`/`n`) and decode it as base-8 before the shortest-form decimal re-emit. Notes: `00` already canonicalises correctly (octal 0 == 0). `08`/`09` are NOT legacy octal (they contain a non-octal digit); upstream rejects them as parse errors, so they never appear as byte-identity inputs and need no handling. Modern `0o17` octal is already handled. Hex/bin/BigInt are unaffected.

### gap-106 — numeric float property key not normalised to string key (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.129. `minify_obj_numkey_float` now ENFORCED.
- **Resolution:** `{.5:1}` → `{"0.5":1}` discovered to already produce byte-identical output (silently fixed by earlier gap work). `minify_obj_numkey_float` removed from `IGNORE_FIXTURES` in CLOC12.129.
- **Input:** `x={.5:1};` → **Upstream:** `x={"0.5":1};`. A non-integer NUMERIC property key is canonicalised by upstream to its string form and quoted: the float key `.5` becomes the string key `"0.5"` (the ToString of the numeric property name). closurec keeps the raw numeric token `.5`.
- **What it needed:** object-key-specific number→string canonicalisation. Only non-integer numeric keys diverge — integer numeric keys (`{1:2}`) are already byte-identical (both keep `1`), so the rule is: when a property key is a numeric literal whose value is not a non-negative integer in canonical form, replace it with the quoted ToString of its numeric value.

### gap-107 — fractional float trailing-zero strip (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.110 (discovered CLOC14.50). `minify_num_frac_trail_zero` / `minify_num_frac_trail_zeros` / `minify_num_frac_lead_zero` now ENFORCED.
- **Fix:** added a gap-107 arm to `normalize_number_value`'s fractional fallback (reached only after `decimal_float_as_u128` returns `None`, i.e. the value is genuinely non-integer): when the separator-stripped literal has a `.` and NO exponent, strip trailing `0`s from the fractional part (and a now-bare trailing `.`), then elide a lone `0` integer part (`0.x` → `.x`; `10.x` keeps its `10`). Pure decimal-string normalisation — the value is exactly representable as written, so no Grisu/Ryu is needed. As a bonus the long-standing `0.5` → `.5` (gap-082's deferred "fractional left verbatim", `gap082_fractional_left_verbatim` → `gap082_fractional_leading_zero_elided`) now resolves too. The genuinely Grisu-needing residuals stay gap-085: exponent forms (`5e-3`, `1e-5`, `0.0001`'s `1E-4`) are excluded by the no-`e`/`E` guard, and f64 precision loss (`12345678901234567890` → `1.2345678901234567E19`) never reaches the arm (all-digits, no `.`). Eight `gap107_*` unit tests + the updated `gap082_*` test cover the strip cases and every non-regression guard (`1.5`/`1.05`/`2.0`/`2.00`). Verified byte-identical to upstream Closure v20240317.
- **Input:** `x=1.50;` → **Upstream:** `x=1.5;` but **closurec:** `x=1.50;`. A FRACTIONAL (non-integer-valued) float literal with trailing zeros in its fractional part keeps them verbatim; upstream strips them to the shortest exact decimal: `1.50` → `1.5`, `1.500` → `1.5`, `3.140` → `3.14`, `10.20` → `10.2`, `123.4500` → `123.45`. The leading-`0` variant compounds with gap-085's leading-zero elision: `0.50` → `.5`, `.50` → `.5`.
- **What it needs:** in `normalize_number_value`, for a literal that has a `.` and a non-integer value (so the gap-082 u128/integer path does not apply) and NO exponent, strip trailing `0`s from the fractional part (and a now-bare trailing `.`), then apply the existing leading-`0` elision. This is tractable WITHOUT Grisu/Ryu because the value is exactly representable as written — it is pure decimal-string normalisation, not a float round-trip. Distinct from gap-082 (integer-valued floats `2.0`/`100.00`, already handled) and from gap-085 (the genuinely Grisu-needing cases: f64 precision loss like `12345678901234567890` → `1.2345678901234567E19`, and scientific↔fractional conversions like `0.0001` → `1E-4`, `5e-3` → `.005`). A combined-exponent case like `1.230e1` → `12.3` overlaps gap-085 (scientific→decimal) and is left there.

### gap-108 — do-body single-statement block flatten (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.111 (discovered CLOC14.51). `minify_do_body_flatten` now ENFORCED.
- **Input:** `do{x()}while(a);` → **Upstream:** `do x();while(a);` (was **closurec:** `do{x()}while(a);`). A do-while loop whose body is a single-statement block has the braces removed, the same flattening upstream already applies to other single-statement bodies. A MULTI-statement body (`do{x();y()}while(a)`) is left braced (correct).
- **Fix:** added a gap-108 token-re-stitcher block in `whitespace_only.rs`, a direct sibling of the gap-080 else-body flatten. Anchor on a `do` keyword (reserved — so `do{…}` is unambiguously the loop body, never an object literal or labelled block), scan the body `{…}` to its matching `}`, and if it holds exactly one statement (no nested `{`, no control-flow keyword at depth 1, zero top-level `;`), drop the braces and replace the `}` with a synthetic `;`. The trailing `while(cond)` is untouched. Multi-statement and empty bodies keep their braces; a body containing a nested control-flow keyword keeps braces (valid output, more-aggressive flatten deferred). Six `gap108_*` unit tests + two updated property-key-safety tests (`gap033`/`gap034`, whose do-bodies now correctly flatten). Verified byte-identical to upstream Closure v20240317. The empty `do{}while(a)` → `do;while(a)` case (closurec emits a stray space `do; while`) is a separate spacing nit left for follow-up.

### gap-109 — string method key normalised to computed key (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.112 (discovered CLOC14.51). `minify_class_string_method` / `minify_obj_string_method` now ENFORCED.
- **Input:** `class A{"m"(){}}` → **Upstream:** `class A{["m"](){}}` (was **closurec:** `class A{"m"(){}}`). A method whose key is a STRING LITERAL is rewritten to a COMPUTED key (`["m"]`), in both class bodies and object literals (`{"m"(){}}` → `{["m"](){}}`). closurec kept the raw string-literal key.
- **Fix:** a gap-109 pre-pass in `whitespace_only.rs` wraps the string in a synthetic `[`…`]` pair when it is a method KEY. Detection mirrors `get_set_computed_needs_space`'s property-start + method-body guards: the string is at a property-start position (preceded by `{`/`,`/`}`/`static`, not a `.`/`?.` member access), is immediately followed by `(` (the parameter list), AND the `)` matching that `(` is immediately followed by `{` (the method body). The method-body guard is the decisive disambiguator: a string CALLED as a function (`"m"(x);`) has its `)` followed by `;`/operator/EOF, never `{`, so it is rejected; a string property VALUE (`{"a":1}`) has `:` after the string, not `(`, so it never reaches the `(` test. Identifier keys (`{m(){}}`), already-computed keys (`{["m"](){}}`), and call arguments (`f("m")`) are all untouched. Eight `gap109_*` unit tests cover the wrap cases and every non-regression guard. Verified byte-identical to upstream Closure v20240317. NOTE: a string-keyed ACCESSOR (`get"a"(){}` → `get "a"(){}`) is a SEPARATE space-insertion gap (upstream inserts a space, does NOT wrap in `[...]`), left for follow-up.

### gap-110 — modifier-prefixed string method key not normalised to computed (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.113 (v0.117.0). `minify_gen_string_method` / `minify_async_string_method` / `minify_async_gen_string_method` now ENFORCED.
- **Input:** `x={*"m"(){}};` → **Upstream:** `x={*["m"](){}};` but **closurec:** `x={*"m"(){}};`. A string method KEY preceded by a method MODIFIER (`*` generator or `async`) is normalised to a COMPUTED key just like the plain case (gap-109), but gap-109's pre-pass only fired when the string's predecessor was a property boundary (`{`/`,`/`}`/`static`), so a `*`/`async`-prefixed key was missed: `{*"m"(){}}` → `{*["m"](){}}`, `class A{async"m"(){}}` → `class A{async["m"](){}}`, `{async*"m"(){}}` → `{async*["m"](){}}`. (`static"m"` already works — gap-109 covered `static`.)
- **Fix:** the gap-109 pre-pass now walks BACK over the contiguous run of method modifiers (`*`, `async`, `static`) from the string's predecessor to the ANCHOR — the token opening the member position — and accepts the key when that anchor is a property-start (`{`/`,`/`}`). This proves the leading `*`/`async` is a method modifier and not a multiply/identifier in an expression: `a=async*b` never matches (`b` is not a string) and `a*"m"(){}` is rejected because the anchor walk lands on the identifier `a` (not a property-start), so the generator/multiply ambiguity is resolved without a spurious `[...]` wrap. The same method-body guard as gap-109 applies (the `)` matching the key's `(` must be followed by `{`). Seven `gap110_*` unit tests cover the generator/async/async-generator/class-member wrap cases plus the `{*m(){}}`, `a=async*b`, and `a*"m"(x)` non-regressions. Verified byte-identical to upstream Closure v20240317.

### gap-111 — reserved keyword before string literal missing separating space (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.114 (v0.118.0). `minify_case_string_space` / `minify_accessor_string_key` / `minify_new_string_callee` now ENFORCED.
- **Input:** `switch(x){case"a":b()}` → **Upstream:** `switch(x){case "a":b()}` but **closurec:** `switch(x){case"a":b()}`. A reserved keyword immediately before a string literal that the keyword grammatically takes needs a separating space that closurec omits: `case"a":` → `case "a":` (case clause), `{get"a"(){}}`/`{set"a"…}` → `get "a"`/`set "a"` (string-keyed accessor — the case noted under gap-109), `new"s"` → `new "s"` (new callee). NOT all keyword+string pairs need it: `typeof"s"`, `void"s"`, `throw"e"`, `a in"s"` are already byte-identical (no space).
- **Fix:** a `keyword_string_needs_space(kept, idx)` helper wired into the emit-time separator OR-chain in `whitespace_only.rs`. It returns true when the current token is a string literal and the previous token is a word-like keyword in the EXACT set `{case, get, set, new}` — inserting one space. The set is exact: `typeof`/`void`/`throw`/`in`/`instanceof` before a string stay adjacent (verified against the JAR). SAFE because in valid JS a bare `KEYWORD"string"` adjacency only occurs in these grammatical positions (two adjacent primaries are a syntax error; these words as property keys/values are always separated from a string by `:`/`(`). Nine `gap111_*` unit assertions cover the four wrap cases plus the excluded-keyword and keyword-as-key/identifier non-regressions. Verified byte-identical to upstream Closure v20240317.

### gap-112 — for-await-of header (bare-body) emits spurious await-before-paren space (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.115 (v0.119.0). `minify_for_await_bare_stmt` now ENFORCED.
- **Input:** `async function f(){for await(const x of y)z()}` → **Upstream:** `async function f(){for await(const x of y)z()};` but **closurec:** `async function f(){for await (const x of y)z()};`. In a `for await(...)` loop header whose body is a BARE statement or declaration head, closurec inserts a SPURIOUS SPACE between the `await` keyword and the opening `(`. The `for`/`await` keyword pair is correct; only the `await`-before-`(` adjacency is wrong. The existing passing fixture `minify_for_await_of` (an EMPTY-block body, `for await(x of y){}` → `for await(x of y);`) does NOT exhibit the space — it appears only for bare-statement / declaration loop bodies (observed with `const x`, bare `x`, and a bare `z()` body).
- **Fix:** a one-line FOR-AWAIT guard in `await_operator_needs_space` (the gap-072 helper that forces a space before the `await` unary operator's operand). When the token two before the `(` — i.e. the token before `await` — is the `for` keyword, the space is suppressed, because here `await` is the `for await` async-iteration modifier and the `(` is the loop HEAD, not an operand. EXACT and SAFE: a genuine unary `await(...)` is never preceded by `for`. The empty-block case `for await(x of y){}`, which formerly passed only via the method-name guard (its `)` is followed by `{`), is subsumed. Five `gap112_*` unit assertions cover the const/bare headers plus the unary-`await(a+b)`-keeps-space and empty-block non-regressions. Verified byte-identical to upstream Closure v20240317. NOTE: the for-await loop-body single-statement block flatten (`for await(let x of y){z()}` → `for await(let x of y)z()`, sibling of gap-074) is a SEPARATE gap, left for follow-up.

### gap-113 — negative-exponent / small-fraction number not canonicalised to uppercase-E scientific (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.113 (v0.124.0). `minify_num_neg_exp` / `minify_num_frac_4dp` un-ignored and round-trip byte-identically.
- **Input:** `x=1e-5;` → **Upstream:** `x=1E-5;` but **closurec:** `x=1e-5;`; `x=.0001;` → **Upstream:** `x=1E-4;` but **closurec:** `x=.0001;`. closurec ALREADY canonicalises POSITIVE-exponent and large round-integer literals to uppercase-E scientific (`1e20` → `1E20`, `1000000000000000000` → `1E18`, `100000000000000000000000` → `1E23`), but the NEGATIVE-exponent / small-fraction branch is missing: a small fraction is left in decimal form, and a `1e-N` literal keeps its lowercase `e`. Upstream's number printer chooses the shortest representation and writes the exponent character as uppercase `E`. The choice is STRICTLY-shorter with a decimal-wins tie (`.001` and `1E-3` are equal length → upstream keeps `.001`; `.0001` is one char longer than `1E-4` → upstream switches to `1E-4`).
- **Resolution:** new `small_fraction_shortest_form` helper in `whitespace_only.rs`, wired into `normalize_number_value` before the gap-107 decimal-strip branch (which it subsumes for value `< 1`). Fires only for values in (0, 1): decomposes the source into a coefficient `M` and base-10 exponent `E` (`value == M × 10^E`) from the EXACT digit string (no Grisu/Ryu), builds the leading-zero-stripped decimal and uppercase-`E` scientific forms, and picks the shorter — DECIMAL on a length tie at/above magnitude `1e-3` (Java's `Double.toString` natural form), SCIENTIFIC below. JAR-verified: `1e-5`→`1E-5`, `.0001`→`1E-4`, `1e-3`→`.001`, `5e-1`→`.5`, `1.2e-4`→`1.2E-4`, `120e-3`→`.12`, `2.5e-8`→`2.5E-8`. A magnitude guard (`-324..=308`) leaves out-of-f64-range literals verbatim and prevents a crafted tiny exponent (`1e-2147483648`) from allocating billions of zero bytes (DoS). Values `>= 1` (integers, `1.5`, the scientific-fractional `1.23e1`→`12.3` residual) and sub-normal-boundary f64 rounding fall through unchanged — the remaining deferred true-Ryu work. Five `gap113_*` unit tests.
- **What it needs:** extend `normalize_number_value` (the gap-082/gap-040 scientific-canonicalisation path) to also consider the uppercase-E scientific form for small magnitudes (negative exponent), picking it when strictly shorter than the decimal form, and to uppercase the `e` of an already-scientific negative-exponent literal. Mirror upstream's tie-break (decimal wins on equal length). Sibling of gap-082 (decimal float/scientific integer canonicalisation).

### gap-114 — large non-round integer not canonicalised to lowercase hex when shorter (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.116 (v0.120.0). `minify_num_bigint_hex` now ENFORCED.
- **Input:** `x=123456789012345678;` → **Upstream:** `x=0x1b69b4ba630f350;` but **closurec:** `x=123456789012345678;`. Upstream's number printer emits a large INTEGER in lowercase hexadecimal (`0x…`) when the hex form is shorter than the decimal form (here 18 decimal digits → 17 hex chars). closurec keeps the decimal literal. Round powers of ten still prefer scientific (`1E18`, even shorter); hex wins only for large NON-round integers where both decimal and scientific are long.
- **Fix:** `normalize_number_value` adds a `hex` candidate to the integer shortest-form comparison, at the LOWEST tie-break priority (decimal > cleaned > scientific > hex) so it is chosen only when STRICTLY shortest — verified against the JAR (`4294967295` decimal 10 == hex 10 stays decimal; round powers prefer scientific). The hex candidate is computed over the f64-ROUNDED value (`(n as f64) as u128`) because a JS Number > 2^53 prints its nearest-double hex bits, not the exact source digits (`123456789012345678` → `…350` from the double, not `…34e`). The decimal/scientific forms are deliberately kept over the EXACT integer: upstream uses shortest-round-trip (Ryu) decimal there, which for a clean power of ten reproduces `1E23`; rounding `n` globally would corrupt `scientific_form_of` (the rounded 10^23 is no longer a clean power, which regressed `minify_num_exp_23` → `1E23` during development). The exact-vs-double decimal mismatch for >2^53 integers that PRINT as decimal is the separate deferred Grisu/Ryu gap, unchanged. Six `gap114_*` unit assertions + diff_minify walk-test + the full number-test suite stay green. Sibling/INVERSE of gap-038 (hex→decimal). The fractional/negative-exponent scientific cases (gap-113) remain OPEN.

### gap-115 — regex/division disambiguation: `a/b/c` mis-lexed as a regex (WHITESPACE_ONLY, CORRECTNESS)

- **Status:** RESOLVED by F10 (declarative lexer mode transitions). `div_chain` enforced. (Was HIGH PRIORITY — it corrupted output to non-parseable JS.)
- **Input:** `x=a/b/c;` → **Upstream:** `x=a/b/c;` but **closurec:** `x=a /b/ c;`. A `/` that follows a VALUE-producing token (identifier, number, `)`, `]`, string, etc.) is the DIVISION operator; only after an operator / statement-start / `(` / `,` / etc. does `/` begin a regex literal. closurec's lexer greedily pairs the two slashes of `a/b/c` into a REGEX literal `/b/`, yielding the token stream `a` `/b/` `c` and emitting `a /b/ c` — which is INVALID JS (two adjacent primary expressions with a regex between them). Affects `a/b/c`, `4/2/1`, `a/b+c/d`, `(a)/b/c`; a SINGLE division `a/b` already lexes correctly (only one slash, no regex pairing). This is the classic ASI-free regex-vs-division context problem.
- **What it needs:** the lexer must track whether a `/` is in DIVISION position (after a value-producing token) or REGEX position (after an operator / `(` / `,` / `{` / `;` / keyword / statement start) and only start a regex literal in the latter. Lexer-level (sibling of gap-044 template lexing) — likely a grammar/lexer-callback change, not a token re-stitch. CORRECTNESS, not just byte-identity.
- **F10 resolution:** the `div`/`default` mode table (see gap-092) tracks division-vs-regex position declaratively. Each operand re-establishes division position: `a` (value-producer) → `div`, so the first `/` lexes as DIVISION and returns the lexer to `default`; then `b` → `div`, so the second `/` is again DIVISION. The whole chain `a/b/c` lexes as `a / b / c` and round-trips byte-identically. No hand-written lexer callback.

### gap-116 — canonical numeric string property key not unquoted (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.116 (v0.123.0). `minify_num_str_key` un-ignored and round-trips byte-identically.
- **Input:** `x={"123":1};` → **Upstream:** `x={123:1};` but **closurec:** `x={"123":1};`. A string property KEY that is a CANONICAL non-negative integer (array-index form) is unquoted to a numeric key by upstream: `{"123":1}` → `{123:1}`, `{"0":1}` → `{0:1}`. NOT every numeric-looking string qualifies — `{"01":1}` (leading zero), `{"1.5":1}` (non-integer), `{"123abc":1}` are all kept QUOTED. The discriminator is the canonical round-trip `String(Number(s)) === s` (and the value is a valid array index / safe integer).
- **What it needs:** an emitter/token-level rule: a STRING literal that is a property KEY (followed by `:` in an object-literal property-start position) whose value `s` satisfies `s.chars().all(ascii_digit) && (s == "0" || !s.starts_with('0'))` and parses to a safe integer → emit the bare digits without quotes. Guard against computed keys, method keys, and string VALUES. Sibling of gap-109/gap-110 (string method-key handling).
- **Resolution:** `numeric_string_key_unquoted(kept, idx)` in `whitespace_only.rs`, wired into the string-emit branch — when a string token is in property-key position (prev `{`/`,`, next `:`) and its value is a canonical non-negative integer `< 2^53`, the key is emitted as `normalize_number_value(digits)` instead of a quoted string, so it composes with the scientific/hex shortest-forms (`{"1000":1}` → `{1E3:1}`, `{"123456789012345":1}` → `{0x7048860ddf79:1}`). The cutoff is strictly `< 2^53` (verified against the JAR: `9007199254740991` unquotes, `9007199254740992` = 2^53 stays quoted because `String(Number(s))` no longer round-trips). The position guard excludes the ternary `a?"1":"2"` confound (string preceded by `?`), string VALUES, and `case "1":`. Unit tests `gap116_canonical_integer_string_key_unquoted` + `gap116_scoped_to_canonical_integer_keys`. Float-key counterpart gap-120 (non-integer key → quoted canonical string) remains OPEN.

### gap-117 — `case` + unary-operator operand missing separating space (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.117 (v0.121.0). `minify_case_neg_num` un-ignored and round-trips byte-identically.
- **Input:** `switch(x){case-1:a()}` → **Upstream:** `switch(x){case -1:a()}` but **closurec:** `switch(x){case-1:a()}`. A `case` clause whose operand begins with a prefix UNARY operator (`-`, `+`, `!`, `~`) needs a separating space that closurec omits: `case-1:` → `case -1:`, `case+1:` → `case +1:`, `case!a:` → `case !a:`, `case~a:` → `case ~a:`. (`case 1:` with a plain number / `case"a":` — gap-111 — are the already-handled or separately-tracked cases.) The glued forms `case-1` etc. are still valid JS (the `-` is unambiguously unary after `case`), so this is a byte-identity gap, not a correctness bug.
- **What it needs:** extend the gap-111 `keyword_string_needs_space` family (or `needs_separator`) so that the `case` keyword immediately followed by a PUNCTUATOR that starts a unary expression (`-`/`+`/`!`/`~`) also takes a separating space. Verify the exact operator set against the JAR; confirm `case(` (parenthesised) and `case identifier` already round-trip.
- **Resolution:** added `case_unary_needs_space(kept, idx)` to `whitespace_only.rs` and wired it into the emit-loop separator OR-chain. Returns true exactly when `kept[idx]` is a structural `-`/`+`/`!`/`~` punctuator and `kept[idx-1]` is the word-like keyword `case`. Probed against the JAR to confirm the rule is `case`-SPECIFIC: `return-1`, `throw-1`, `typeof-1`, `void-1`, `a in-1` all stay GLUED (closurec already matches the JAR), only `case`+unary takes the space. `case(1)`→`case 1` paren-drop is a SEPARATE out-of-scope gap. Unit tests `gap117_case_unary_operand_space` + `gap117_scoped_to_case`.

### gap-118 — retained-hex literal not lowercased (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.118 (v0.122.0). `minify_hex_upper_retained` un-ignored and round-trips byte-identically.
- **Input:** `var n=0xFFFFFFFFFFFFF;` → **Upstream:** `var n=0xfffffffffffff;` but **closurec:** `var n=0xFFFFFFFFFFFFF;`. When a hex literal's hexadecimal form is the SHORTEST representation (so the number printer keeps it in hex rather than converting to decimal/scientific), upstream emits the digits in LOWERCASE. closurec leaves the original uppercase digits. Inverse/sibling of gap-114 (decimal → lowercase hex when shorter, RESOLVED): there the hex was synthesised lowercase via `format!("0x{:x}")`; here the hex is the SOURCE form and is passed through verbatim. Small hex values that convert to decimal (`0xFF` → `255`, `0xabc` → `2748`) are unaffected — only literals large enough that hex stays shortest expose the case mismatch.
- **Root cause:** in `normalize_number_value`'s shortest-form comparison, the original literal becomes the `cleaned` candidate at its source case (`0xFFFFFFFFFFFFF`, 15 chars) and the gap-114 lowercase-hex candidate (`0xfffffffffffff`, 15 chars) ties it on length; the tie-break order (decimal > cleaned > scientific > hex) lets the verbatim uppercase `cleaned` form win, so the uppercase survives.
- **Resolution:** lowercase the `cleaned` candidate when it is a `0x`/`0X` literal (`cleaned.to_lowercase()`) before the shortest-form comparison, so the verbatim and synthesised-hex candidates are byte-identical and either wins the tie correctly. Scoped to hex — decimal/octal/binary cleaned forms have no case-significant letters (scientific `e`/`E` is the gap-082 path, small octal/binary never stay in radix form). Unit tests `gap118_retained_uppercase_hex_lowercased` + `gap118_decimalising_hex_unaffected`; verified vs the JAR that `0xFF`/`0xAbC` still decimalise, already-lowercase retained hex is unchanged, and gap-114 decimal→hex emission stays lowercase.

### gap-119 — spurious space between `return` keyword and regex literal (WHITESPACE_ONLY)

- **Status:** RESOLVED by F10 + a `needs_separator` refinement. `regex_after_return` enforced.
- **Input:** `function f(){return/a/g}` → **Upstream:** `function f(){return/a/g};` but **closurec:** `function f(){return /a/g};`. A regex literal immediately following the `return` keyword gets a spurious separating space. `return/a/g` is valid JS (a regex is expected in expression position after `return`), so upstream glues them; closurec's separator logic inserts a space between the word-like `return` token and the `/`-led regex token.
- **Family / risk:** regex/division disambiguation family — sibling of gap-115 (`a/b/c` mis-lexed as regex, CORRECTNESS). The space here is harmless (output stays valid) but non-byte-identical.
- **Resolution:** two parts. (1) F10's `default`-mode rule keeps `return` in regex position, so `/a/g` lexes as a SINGLE `REGEX` token (no longer split into `/a/` + `g`). (2) `needs_separator` in `whitespace_only.rs` gained a short-circuit: a `REGEX` literal as the RIGHT token never needs a leading separator from a word-like token, because a regex's first emitted character is `/` (a punctuator) — `return/a/g`, `typeof/x/`, `x in/re/` all join cleanly. The lone hazard (the previous token's emitted text ENDING in `/`, which would glue into a `//` line comment) is guarded: only a `/` division operator or a flagless `/x/` regex can do that, and we fail safe and keep the space for them. New helper `is_regex(tok)` (grammar `type_name == "REGEX"`); unit tests `gap119_regex_after_return_no_space` + `gap119_regex_after_assign_preserved`.

### gap-120 — non-integer numeric property key not quoted/canonicalised (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.120 (v0.125.0). `minify_float_key_quoted` un-ignored and round-trips byte-identically.
- **Input:** `x={.5:1};` → **Upstream:** `x={"0.5":1};` but **closurec:** `x={.5:1};`. A NON-INTEGER numeric property key is emitted by upstream as a QUOTED canonical-number string: `{.5:1}` → `{"0.5":1}`, `{1.5:1}` → `{"1.5":1}`, `{1.50:1}` → `{"1.5":1}`, `{1e-3:1}` → `{"0.001":1}`. Float-key counterpart of gap-116 (canonical INTEGER string key → unquoted number). Upstream canonicalises every property key to `String(Number(key))` and then emits it bare iff that string is a valid numeric/identifier key, else quoted. INTEGER numeric keys already round-trip in closurec (`{5:1}` stays, `{0xff:1}` → `{255:1}`, `{1e3:1}` → `{1E3:1}` since 1000 is a safe integer); only the non-integer (fractional / negative-exponent) keys diverge.
- **What it needs:** an emitter/token-level rule recognising a NUMBER token in property-key position (`{` or `,` then NUMBER then `:`) whose value is NON-INTEGER; replace it with a double-quoted string of its canonical decimal (`String(Number(v))`). Simple fixed-point decimals (`.5` → `0.5`, `1.5` → `1.5`, trailing-zero strip `1.50` → `1.5`) are tractable with the existing `normalize_number_value` decimal canonicalisation; the negative-exponent/tiny-fraction canonical form (`1e-3` → `0.001`) overlaps the deferred gap-113 (Ryu) number-printer work, so a first slice can scope to plain decimals and leave `1e-N` keys for the gap-113 follow-up. Guard against integer keys (already correct), computed keys (`[expr]`), string keys, and method/accessor keys.
- **Resolution:** `noninteger_numeric_key_string` helper in `whitespace_only.rs`, wired into the number-emit branch. Builds JS `String(Number(key))` EXACTLY from the source digits (coefficient `M`, base-10 exponent `E`) — leading `0` KEPT before the point, trailing fractional zeros stripped, lowercase-`e` exponential only for magnitudes below `1e-6` (`sci_exp <= -7`). This is a DISTINCT algorithm from closurec's value number printer (which drops the leading `0` and uses uppercase-`E`), so the full `1e-N` range was handled directly (no gap-113-Ryu dependency in the end — both are exact string transforms). Verified against the JAR: `{1e-6:1}` → `{"0.000001":1}` stays decimal, `{1e-7:1}` → `{"1e-7":1}` exponential. Integer keys (`E >= 0` after trailing-zero stripping) stay bare. The position guard (prev `{`/`,`, next `:`) reuses gap-116's and excludes the ternary `a?1.5:2` confound, array/call elements, bare values, and the value half of a `{key:value}` pair (`{1.5:.5}` → `{"1.5":.5}`). f64-range magnitude guard (`-324..=308`) bounds the zero-run (no DoS). Two `gap120_*` unit tests.

---

## CLOC12.132 — correlation-vector gap-drop tombstones in `whitespace_only_minify`

- **Status:** RESOLVED in CLOC12.132 (v0.132.0).
- **What it was missing:** The CV sidecar recorded tombstones for trivia/EOF
  tokens (via `whitespace_only_dropped` in `run.rs`) but was silent about
  non-trivia tokens removed by gap pre-passes. For example, gap-053 drops the
  redundant parentheses from `var x=(1);` → `var x=1;`; before this slice,
  the `(` and `)` token CVs had no deletion record.
- **Resolution:**
  - `whitespace_only_minify` gains a third parameter:
    `cv: Option<(&mut CVLog, &str, &[String])>` — (log, file_cv_id,
    token_cv_ids). When `None`, behaviour is byte-identical to before.
  - After all pre-passes complete (`let kept = kept`), a pointer-comparison
    sweep finds non-trivia, non-EOF tokens from the original stream absent
    from `kept` and issues `CVLog::delete(cv_id, "whitespace_only", "gap_drop",
    {token_index, lexeme})` for each.
  - `transform_source_with_cv` signature updated: `cv` tuple gains
    `&[String]` (per-token CV ID slice); callers passing `Some(...)` now
    include `token_cv_ids` (hoisted in `run_compiler` from the lex block).
  - Two new tests in `run.rs` pin the contract.
- **Scope:** covers pre-pass drops only. Emit-loop drops are covered in
  CLOC12.133 (v0.133.0).

## CLOC12.133 — correlation-vector emit-loop skip tombstones in `whitespace_only_minify`

- **Status:** RESOLVED in CLOC12.133 (v0.133.0).
- **What it was missing:** CLOC12.132 tombstoned tokens dropped by gap
  *pre-passes* but not tokens in `kept` that the emit loop suppresses.
  Seven skip sites in the emit loop had no CV record.
- **Resolution:**
  - `whitespace_only_minify` parameter changed from `cv` to `mut cv` to
    allow reborrowing.
  - A `ptr_to_cv_id: HashMap<*const Token, String>` is built once before
    the emit loop (O(n) setup, O(1) lookup per site).
  - The pre-pass sweep (CLOC12.132) now uses `cv.as_mut()` (borrow, not
    consume) so `cv` is available in the emit loop phase.
  - A `tombstone_emit_skip` closure is defined with access to both
    `emit_cv: Option<&mut CVLog>` and `ptr_to_cv_id`.
  - **Seven emit-loop sites tombstoned:** gap-050 (empty `new X()` parens),
    gap-030-rule-a (`;` before `}`), gap-030-rule-c (dedup `;` after
    synthetic `;`), gap-046 (trailing array `,`), gap-046b (trailing object
    `,`), gap-032 (`{`/`}` in flatten path), gap-031 (`{}`→`;`
    substitution).
  - All tombstones: `source="whitespace_only"`, `reason="emit_skip"`,
    `meta.gap=<rule>`, `meta.lexeme=<original value>`.
  - Two new integration tests in `run.rs` pin gap-050 and gap-030-rule-a.

## CLOC12.134 — close gap-049 (pinning fixture + dead-assignment cleanup)

- **Status:** RESOLVED in CLOC12.134 (v0.134.0).
- **What it was missing:** gap-049 (trailing `;` suppression when gap-032
  flattens a block whose `}` is immediately before an outer `}`) was
  implemented implicitly in CLOC12.56 via `drop_trailing_semi`, but the
  spec retained a stale OPEN entry and no dedicated byte-identity fixture
  existed to pin the behaviour.
- **Resolution:**
  - Removed the duplicate OPEN gap-049 entry from the spec; merged into
    a single RESOLVED entry with accurate attribution to CLOC12.56 +
    CLOC12.134 pinning.
  - Added `tests/diff/minify_for_body_inner_close/` fixture:
    input `async function f(){for await(var v of a){a;}}`,
    expected `async function f(){for await(var v of a)a};`.
    This locks the `drop_trailing_semi` path so any regression in the
    gap-032 next-after-close peek will be caught by CI.
  - Removed dead intermediate assignment `prev_emitted_tok = Some(ident)`
    in the gap-045 arrow-paren elision arm (line was immediately overwritten
    by `prev_emitted_tok = Some(kept[idx + 3])`; only a compiler warning,
    no correctness impact).

## CLOC12.135 — close gap-044 (first slice resolved, gap-044b follow-up documented)

- **Status:** RESOLVED in CLOC12.135 (v0.135.0).
- **What it was missing:** gap-044 was marked OPEN in the spec even though the
  first slice (simple-identifier substitutions `${name}`, `${x}`) was already
  resolved by the F10 declarative lexer mode work. Both `minify_template_subst`
  and `minify_tagged_subst` fixtures pass. The spec had a stale OPEN entry and
  no documentation of the residual limitation.
- **Resolution:**
  - gap-044 entry updated to RESOLVED (first slice).
  - New gap-044b entry added documenting the open residual: expressions with
    operators (`.`, `+`, `(`, …) or nested `{}` inside `${…}` trip the
    div/default mode reset. Root cause: the F10 mode table lacks brace-depth
    tracking, so `}` inside `${a.b}` reads as a plain RBRACE rather than a
    `TEMPLATE_TAIL`, raising `LexerError: Unexpected sequence '` `` ` `` `'`.
  - gap-044b states the correct fix: an explicit mode stack in `GrammarLexer`
    (push template mode on `${`, pop on matching `}`).

## CLOC12.136 — port `RemoveUnusedCodeTest` into `closure-pass-remove-unused-vars`

- **Status:** port landed; 11 active `#[test]`s pass, 6 `#[ignore]` gaps opened.
- **What it is:** the fourth CLOC12 upstream port, covering the unused-binding
  removal that `RemoveUnusedCode` (formerly `RemoveUnusedVarsTest`) performs.
  `closure-pass-remove-unused-vars/tests/upstream/remove_unused_vars_test.rs`
  pins the provably-sound core our `RemoveUnusedVarsPass` implements today:
  GLOBAL-scope `var`/`let`/`const` bindings that are unreferenced and have a
  pure initializer (literal, bare identifier, or none) are removed; multi-
  declarator declarations are split to keep the survivors; impure initializers
  keep the binding. All 11 active cases pass — the port confirms the pass is
  sound on its covered surface and adds canonical upstream coverage. No live
  defect surfaced this round.
- **New gaps** (upstream behaviors our narrow pass does not cover yet; each has
  an executable `#[ignore = "blocked on gap-NNN"]` placeholder that goes live
  when the gap closes):
  - **gap-121** — function-local unused-var removal. The pass restricts removal
    to `ScopeId::GLOBAL`; nested-scope name handling is a follow-up (the scope
    analyzer surfaces the bindings but the apply step only matches top-level
    `program.body` names).
  - **gap-122** — unused function-declaration removal. `Function`-kind bindings
    are filtered out at the eligibility scan; dropping an unreferenced
    `function g(){}` is the treeshake pass's job, not this one.
  - **gap-123** — unused function-parameter removal (`function f(a,b){return a}`
    → drop trailing `b`). `Param`-kind bindings are skipped; needs arity-aware
    param analysis.
  - **gap-124** — side-effect extraction: `var a = f();` (a unused) should
    become a bare `f();`, preserving the initializer's effect while dropping
    the binding. We conservatively keep the whole binding (the purity gate
    refuses to delete a call initializer). Needs the initializer lifted to an
    `ExpressionStatement` in the apply step.
  - **gap-125** — self-referential dead binding: `var a = function(){a()};`
    with no external use should be removed. A naive use-count sees `a`
    referenced by its own body and keeps it; needs reference-cycle detection
    (SCC over the binding→reference graph).

## CLOC12.138 — activate the stale emitter-conformance placeholders (gap-025/026/027) + open gap-133

- **Status:** three previously-`#[ignore]`d emitter conformance tests are now
  **active** and passing; one new gap opened.
- **What it was:** gap-025 (number shortest-form), gap-026 (string quote-choice),
  and gap-027 (precedence-aware paren insertion) were all RESOLVED in the emitter
  (CLOC12.10/11/12) but their `closure-emitter/tests/upstream/code_printer_test.rs`
  placeholders were left `#[ignore]`d "to be re-port'd in a follow-up". This is
  that follow-up: the three placeholders now carry real byte-equal assertions
  against the emitter's actual output, converting them from documentation stubs
  into executable conformance coverage:
  - `test_number_formatting_shortest_form` — `1000000000`→`1E9`, `1000000`→`1E6`,
    `1e21`→`1E21`, `100`→`100` (tie→decimal), `0.5`→`0.5`, `-0.0`→`-0`.
  - `test_string_quote_choice_minimises_escapes` — `she said "hi"`→
    `'she said "hi"'` (single-quote), `o'malley`→`"o'malley"`, `plain`→`"plain"`.
  - `test_operator_precedence_inserts_inner_parens` — `a*(b+c)`, `(a+b)*c`, and
    `a+b*c` (no parens where `*` already binds tighter).
- **gap-133 (surfaced here)** — leading-zero drop for fractional literals.
  Upstream Closure emits `.5` for `0.5` (and `-.5` for `-0.5`), dropping the
  redundant leading zero; `format_js_number` keeps `0.5` because its
  decimal-vs-exponential comparison never strips the leading `0`. A conservative
  miss (both are valid, same-value JS), **not** a miscompile. Fix: after picking
  the decimal spelling, strip a leading `0` before `.` (and after a `-`). Small,
  self-contained follow-up in `format_js_number`.
  - **gap-126** — assignment-only dead var: `var a; a = 1;` (never read) should
    be removed. The analyzer counts the `a = 1` write as a reference, so `a`
    survives; needs write-vs-read reference classification so pure writes to an
    otherwise-unread binding don't keep it alive.

## CLOC12.137 — port `InlineFunctionsTest` into `closure-pass-inline`

- **Status:** port landed; 7 active `#[test]`s pass, 6 `#[ignore]` gaps opened.
- **What it is:** the fifth CLOC12 upstream port, covering the function-body
  substitution that `InlineFunctions` performs.
  `closure-pass-inline/tests/upstream/inline_functions_test.rs` drives the real
  `source → bridge → inline → emit` chain and asserts on the emitted string. It
  pins the sound core our `InlinePass` implements: substitute a `return <expr>;`
  body at its call site(s) — single-use always, multi-use under a size budget —
  when every argument is a simple leaf (identifier/literal) and the body has no
  free identifiers beyond the parameters. 7 active cases pass (zero-param
  constant/string returns, two-site inlining, member-object substitution,
  nested-in-binary, decline-non-call-use, decline-over-budget-multi-use).
- **New gaps** (executable `#[ignore = "blocked on gap-NNN"]` placeholders):
  - **gap-127** — inlining a function with local declarations (`var`/`let` in
    the body); the slice handles only a single `return <expr>;`.
  - **gap-128** — inlining a method that references `this`; the slice bails on
    any free identifier.
  - **gap-129** — inlining a function *expression* / arrow bound to a variable
    (`var f = function(x){…}`); the slice recognizes only `function`
    declarations.
  - **gap-130** — inlining a void (no-return) function called for its side
    effect only; the slice targets value-position `return` bodies.
  - **gap-131** — no dedicated recursion guard: a self-referential callee is
    declined today only because its body's free self-reference fails the
    no-free-identifier gate; pinned so widening that gate can't silently start
    inlining a recursive body.
  - **gap-132 (surfaced by this port)** — a **compound (non-leaf) argument**
    expression is declined rather than inlined. `function d(x){return x*2}
    g(d(a+b));` is left as `g(d(a+b));`; upstream inlines it to `g((a+b)*2);`.
    Our slice substitutes only simple (identifier/literal) arguments. This is a
    conservative miss, **not** a miscompile. The fix needs: (1) allow a compound
    argument when its parameter is used exactly once in the body (so it isn't
    duplicated / re-evaluated), and (2) parenthesize the substituted expression
    against the surrounding operator's precedence. Candidate for a dedicated
    follow-up fix PR.

## CLOC12.139 — port `RenameVarsTest` into `closure-pass-rename-globals`

- **Status:** port landed; 8 active `#[test]`s pass, 4 `#[ignore]` gaps opened.
- **What it is:** the sixth CLOC12 upstream port. `closure-pass-rename-globals`
  exposes everything a source-string surface needs through public crate APIs,
  so — unlike the AST-builder ports (dce, remove-unused-vars) — this port drives
  the real `source → bridge → RenameGlobalsPass → emit` chain and asserts on the
  emitted string, exactly as upstream `RenameVarsTest`'s `test(js, expected)`.
  It pins the sound global slice our `RenameGlobalsPass` implements: rename
  top-level `function` names and `var`/`let`/`const` targets to the shortest
  fresh names `a`, `b`, `c`, … in first-appearance order, leaving untouched
  names already one character long, free/undeclared globals, dotted property
  keys, and any do-not-rename extern. 8 active cases pass (two-globals→`a`/`b`,
  all-uses-rewritten, function-decl rename, reserved-extern-skipped-but-ordinary-
  global-renamed, free-global-untouched, dotted-key-untouched, computed-member-
  index-renamed, single-char-not-lengthened).
- **No new closurec bug surfaced** — one expectation was corrected during the
  port (the reserved-extern case renames the *ordinary* global `helper`→`a`
  while keeping only the reserved `apiHandler`, which is the correct behavior).
- **New gaps** (executable `#[ignore = "blocked on gap-NNN"]` placeholders):
  - **gap-134** — rename **function-local** variables. Upstream `RenameVars`
    shortens locals too; our pass only renames globals, so a body-local
    `var innerLongName` is left as written.
  - **gap-135** — rename **function parameters**. Upstream shortens parameter
    names; our pass leaves them untouched.
  - **gap-136** — **reuse** a freed short name across two disjoint local scopes
    (both locals may become `a`). Our global-only pass never allocates local
    names, so it cannot reuse them.
  - **gap-137** — **pseudo-name / stable-name mode**: upstream can map each
    original name to a stable human-readable placeholder instead of a minimal
    short name. Our pass has only the minimal-short-name mode.

## CLOC12.140 — port `RenamePropertiesTest` into `closure-pass-rename-properties`

- **Status:** port landed; 8 active `#[test]`s pass, 3 `#[ignore]` gaps opened.
- **What it is:** the seventh CLOC12 upstream port. Like the `rename-globals`
  port, `closure-pass-rename-properties` exposes a source-string surface through
  public crate APIs, so this port drives the real
  `source → bridge → RenamePropertiesPass → emit` chain and asserts on the
  emitted string, exactly as upstream `RenamePropertiesTest`'s
  `test(js, expected)`. It pins the sound name-based slice our pass implements:
  rename dotted, unquoted property names (member accesses and object-literal
  keys) to the shortest fresh names `a`, `b`, `c`, … in first-appearance order,
  consistently across every occurrence, leaving untouched a name accessed via a
  quoted/computed subscript anywhere, a single-character name, a curated set of
  built-in / DOM names, and any externs entry. 8 active cases pass (consistent
  private-property rename, reads-and-object-literal-keys collapse, distinct-name
  assignment down a member chain, quoted-access-poisons-rename, built-in-names-
  untouched, single-char-untouched, computed-index-untouched, externs-preserved).
- **No new closurec bug surfaced** — every active expectation matched the pass
  on the first run.
- **New gaps** (executable `#[ignore = "blocked on gap-NNN"]` placeholders):
  - **gap-138** — **type-/heap-aware disambiguation**: upstream can rename the
    same property name on two unrelated object types to two different short
    names. Our pass renames a name once, globally.
  - **gap-139** — **frequency-ordered short-name assignment**: upstream packs
    the most-used property into the shortest name; our pass assigns by first
    appearance regardless of usage count.
  - **gap-140** — **cross-module shared rename map**: upstream can keep a
    property rename stable across separately-compiled modules via a shared map;
    our single-program pass has no such map.

## CLOC12.141 — port `PeepholeReplaceKnownMethodsTest` into `closure-pass-constant-fold`

- **Status:** port landed; 10 active `#[test]`s pass, 3 `#[ignore]` gaps opened.
- **What it is:** the eighth CLOC12 upstream port and the second into
  `closure-pass-constant-fold` (alongside the `PeepholeFoldConstants` port). It
  drives the crate's AST-builder surface (the pass keeps a minimal
  dev-dependency set, so — like the sibling port — cases are hand-built call
  expressions, not source strings) and pins the String-method folds our
  `ConstantFoldPass` performs today: `indexOf`, `lastIndexOf`, case conversion,
  `slice`, `substring`, `substr`, `charAt`, `charCodeAt`, `repeat`, `trim`, and
  the boolean `includes`/`startsWith`/`endsWith`, plus the decline on a
  non-constant receiver. All 10 active cases pass on the first run.
- **No new closurec bug surfaced** — every fold matched the pass exactly.
- **New gaps** (executable `#[ignore = "blocked on gap-NNN"]` placeholders):
  - **gap-141** — ~~fold `Math.abs`/`floor`/`ceil`/`round` on numeric
    literals~~ **RESOLVED** (constant-fold 0.79.0, PR #7217; folds all four
    with round-half-toward-`+Infinity` and negative-zero declines). The
    `folds_math_unary_methods` placeholder is now an **active** conformance
    test (also covers `ceil`/`round`) as of constant-fold 0.80.0.
  - **gap-142** — ~~fold `Array.prototype.join` on an array literal of
    constants (`[a,b,c].join("-")` → `"a-b-c"`)~~ **RESOLVED** (constant-fold
    0.81.0). Folds when every element is a join-representable constant
    (strings, numbers, booleans, `null`/`undefined`/holes → `""`) and the
    separator is absent (`","`) or a string literal; declines non-constant
    elements, nested arrays/objects, and non-string separators; DoS length cap.
    The `folds_array_join` placeholder is now an **active** conformance test.
  - **gap-143** — ~~fold `String#concat` with **non-string (coerced) args**
    (`"x".concat(1, 2)` → `"x12"`)~~ **RESOLVED** (constant-fold 0.82.0). The
    concat fold now applies `ToString` to primitive-constant arguments
    (number/boolean/`null`→`"null"`/`undefined`→`"undefined"`); objects/arrays/
    non-constants still decline. The `folds_string_concat_with_coerced_args`
    placeholder is now an **active** conformance test. **With this the whole
    `PeepholeReplaceKnownMethods` port is active — gaps 141/142/143 all closed.**

## CLOC12.142 — CodePrinter number-formatting port (emitter)

- **What it is:** the ninth CLOC12 upstream port and the second into
  `closure-emitter` (alongside the CodePrinter core / declarations /
  trailing-comma ports). New file
  `tests/upstream/code_printer_numbers_test.rs`, registered as the
  `upstream_code_printer_numbers` test target. It reshapes upstream
  `CodePrinterTest.java`'s number-printing assertions onto our AST surface
  (a `NumericLiteral` emitted as a single expression-statement) and pins the
  **exponential-vs-decimal cut-over** in `format_js_number`: the emitter forms
  the plain-decimal and the uppercase-`E` exponential spellings and keeps
  whichever is strictly shorter, breaking ties toward decimal.
- **6 active `#[test]`s pass on the first run** (no new emitter bug): `1e18` →
  `1E18`, `1e100` → `1E100`, `2.5e10` → `2.5E10`, `123456789` stays decimal
  (its `1.23456789E8` spelling is longer), `1e-7` → `1E-7`, and `1234.5` stays
  decimal.
- **2 `#[ignore = "blocked on gap-133"]` placeholders** pin the leading-zero
  drop upstream applies to bare fractions (`0.25` → `.25`, `0.125` → `.125`).
  These exercise the **emitter** path (`format_js_number`, AST → string), which
  still keeps the `0` — distinct from the source-preserving byte-identity path
  that already elides it (gap-107 / gap-113). No new gap number is opened; the
  placeholders reference the existing **gap-133** from CLOC12.138.

## CLOC12.143 — CodePrinter string-escape port (emitter)

- **What it is:** the tenth CLOC12 upstream port and the third into
  `closure-emitter` (alongside the CodePrinter core / declarations /
  trailing-comma / numbers ports). New file
  `tests/upstream/code_printer_string_escape_test.rs`, registered as the
  `upstream_code_printer_string_escape` test target. It reshapes upstream
  `CodePrinterTest.java`'s string-escaping assertions onto our AST surface (a
  `StringLiteral` emitted as a single expression-statement) and pins the escape
  sequences `emit_string` / `choose_quote_and_escape` produce.
- **7 active `#[test]`s pass on the first run** (no new emitter bug): backslash
  doubling, the `\n`/`\r`/`\t` short escapes, an "other" control char as
  upper-case `\uXXXX` (`U+0007` → ``), the U+2028/U+2029 line-terminator
  escapes, printable non-ASCII left verbatim in the default (non-`ascii_only`)
  mode, and two single-quote-path cases selected by quote-choice.
- **No `#[ignore]` placeholders** — every case the emitter handles today is an
  active conformance test. (The `ascii_only` `\uXXXX` escaping of printable
  non-ASCII is a separate `EmitOptions` path and is left for a follow-up port.)

## CLOC12.144 — CodePrinter ASCII-only escape port (emitter)

- **What it is:** the eleventh CLOC12 upstream port and the fourth into
  `closure-emitter`. New file `tests/upstream/code_printer_ascii_escape_test.rs`,
  registered as the `upstream_code_printer_ascii_escape` test target. It covers
  upstream `CodePrinterTest.testUnicode`'s `--output_charset=US-ASCII` behavior
  — our `EmitOptions { ascii_only: true }` path (`escape_ascii_only`), the
  distinct sibling of the default-mode string-escape port (CLOC12.143).
- **7 active `#[test]`s pass on the first run** (no new emitter bug): printable
  ASCII verbatim, a Latin-1 accent (`é` → `é`), a BMP CJK ideograph
  (`中` → `中`), an astral code point using the braced form (`💩` →
  `\u{1F4A9}`), a control char (`U+0007` → ``), the named `\n`/`\t`
  escapes still applying, and the `ascii_only`-always-double-quotes rule (a
  value of two `"` prints `"\"\""`, not single-quoted).
- **No `#[ignore]` placeholders.** The ASCII-only `\uXXXX` escaping that the
  CLOC12.143 port noted as a follow-up is now covered here.

## CLOC12.145 — RenameVars port (rename)

- **What it is:** the twelfth CLOC12 upstream port and the **first** into
  `closure-pass-rename`. New file `tests/upstream/rename_vars_test.rs`,
  registered as the `upstream_rename_vars` test target, ported from upstream
  `RenameVarsTest.java`. Unlike the hand-built-AST ports, it drives the real
  source → `grammar_to_program` bridge → `RenamePass` → `emit` roundtrip (the
  crate already carries `javascript-parser` + `closure-emitter` dev-deps), so
  each case is `assert_eq!(rename(src), expected)` on emitted JS.
- **12 active `#[test]`s pass on the first run** (no new rename bug): leaf
  local `var`/`let`/`const` and parameter renaming at declaration + use sites,
  multiple params distinct, param+local together, reserved-name avoidance
  (fresh name never captures a referenced free global), property access and
  non-computed object keys never renamed, computed member index renamed,
  single-char names left alone, and two soundness guards (catch bindings
  reserved; a name declared twice is skipped).
- **4 `#[ignore]` placeholders** pin the whole-program `RenameVars` behaviors
  closurec deliberately does not do in this pass — see gap-144 … gap-147.

### gap-144 — RenamePass does not rename global variables

Upstream `RenameVars` (`testRenameGlobals`) shortens top-level bindings in the
same sweep as locals. closurec **splits** the responsibility: global renaming
lives in the separate `rename-globals` pass (ADVANCED only), so `RenamePass`
leaves a top-level `var longName` untouched. Placeholder:
`rename_global_var`.

### gap-145 — non-leaf (nesting) function params not renamed

`RenamePass` only renames **leaf** functions (no nested function). Upstream
renames every scope. A parameter of a function that contains a nested function
is left verbatim today. Placeholder: `rename_non_leaf_function_param`.

### gap-146 — function declaration names not renamed

Upstream shortens the function *name* itself (`function longName(){}` →
`function a(){}`). closurec preserves declaration names (they may be externally
referenced; renaming them belongs to `rename-globals` with export awareness).
Placeholder: `rename_function_declaration_name`.

### gap-147 — name allocation is declaration-order, not frequency-biased

Upstream's generator (`testBias*`) hands the shortest name to the
most-referenced variable to improve gzip. `RenamePass` allocates fresh names in
declaration order. Placeholder: `frequency_biased_name_allocation`.

## CLOC12.146 — InlineVariables port (inline-variables)

- **What it is:** a CLOC12 upstream port and the **first** into
  `closure-pass-inline-variables`. New file
  `tests/upstream/inline_variables_test.rs`, registered as the
  `upstream_inline_variables` test target, ported from upstream
  `InlineVariablesTest.java`. Drives the real source → `grammar_to_program`
  bridge → `InlineVariablesPass` → `emit` roundtrip (the crate carries
  `javascript-parser` + `closure-emitter` dev-deps), so each case is
  `assert_eq!(propagate(src), expected)` on emitted JS.
- **13 active `#[test]`s pass on the first run** (no new propagation bug):
  single-use const-literal propagation, propagation into an expression, a
  short literal at multiple sites, boolean/null literals, the multi-use size
  budget (long literal declined at multiple sites, propagated at a single
  site), `let`/`var` never propagated, non-literal initializers declined, the
  shadowed-name guard, property names never replaced while computed member
  indices are, and two TDZ soundness cases.
- **3 `#[ignore]` placeholders** pin whole-`InlineVariables` behaviors this
  pass does not do — see gap-148 … gap-150.

  (Numbering note: this port was authored off `origin/main` in parallel with
  the rename port CLOC12.145 / gap-144…147, so it deliberately takes the next
  free numbers CLOC12.146 / gap-148…150 to avoid collision when both merge.)

### gap-148 — single-assignment `let`/`var` not inlined

closurec's `InlineVariablesPass` only propagates `const` bound to a literal.
Upstream `InlineVariables` inlines any *effectively-constant* variable — a
`let`/`var` assigned exactly once and never reassigned — and removes the
declaration. Placeholder: `inline_single_assignment_let`.

### gap-149 — identifier-alias initializers not inlined

Upstream inlines `const A = B;` (an identifier-alias initializer), replacing
uses of `A` with `B` when the alias target is not reassigned. Ours declines any
non-literal initializer. Placeholder: `inline_identifier_alias`.

### gap-150 — dead `const` declaration husk not removed

closurec's pass only *propagates*: after inlining every reference it leaves the
now-dead `const R = 2;` in place for the downstream `remove-unused-vars` pass
to delete. Upstream `InlineVariables` removes the declaration itself once it has
no remaining uses. Placeholder: `removes_dead_const_after_inlining`.

## CLOC12.147 — CodePrinter object-literal port (emitter)

- **What it is:** the fifth CodePrinter port into `closure-emitter`. New file
  `tests/upstream/code_printer_object_literal_test.rs`, registered as the
  `upstream_code_printer_object_literal` test target. It reshapes upstream
  `CodePrinterTest`'s object-literal cases (`testObjectLit*` and the key-quoting
  behaviors) onto our AST surface and pins the `emit_object` /
  `emit_property` / `emit_property_key` output in the default (minified) mode.
- **13 active `#[test]`s pass on the first run** (no new emitter bug): the empty
  object, single/multiple identifier-keyed properties (comma-separated, no
  interior whitespace), string-key quote-stripping (`{"abc":1}` → `{abc:1}`,
  reserved words bared, non-identifier and numeric-looking keys kept quoted, the
  `"__proto__"` exception kept quoted), numeric-literal keys, computed keys
  (`{[a]:1}`), shorthand (`{a}`), a nested object, and a string-valued property.
  Every case shows the object parenthesized at statement start (`({…});`).
- **No `#[ignore]` placeholders.** Getters/setters (`{get a(){}}`) and method
  shorthand (`{m(){}}`) are out of scope — their values are
  `FunctionExpression`s, which the Phase-1 emitter does not yet print and the
  AST cannot fully represent; they join when function-expression emission lands.

  (Numbering note: authored off `origin/main` in parallel with the
  inline-variables port CLOC12.146 / gap-148…150, so this takes CLOC12.147 to
  avoid collision when both merge. This port opens no new gaps.)

## CLOC12.148 — UnreachableCodeElimination port (dce)

- **What it is:** the **second** CLOC12 upstream port into `closure-pass-dce`
  (alongside the `PeepholeRemoveDeadCode` port). New file
  `tests/upstream/unreachable_code_elimination_test.rs`, registered as the
  `upstream_unreachable_code_elimination` test target, ported from upstream
  `UnreachableCodeEliminationTest.java`. Hand-built typed-AST harness (the crate
  has no source-string entry point), asserting surviving function-body
  statements after running only `DcePass`.
- **8 active `#[test]`s pass on the first run** (no new DCE bug): drop
  single/multiple statements after `return`, drop after `throw`, keep reachable
  code before the terminator, bare `return` unchanged, empty-statement removal,
  dead-code cleanup inside a nested `if`-consequent block, and the
  hoisting-soundness decline (a dead tail carrying a `var` is kept verbatim).
- **2 `#[ignore]` placeholders** — see gap-151, gap-152.

  (Numbering note: authored off `origin/main` in parallel with the emitter
  object-literal port CLOC12.147, so this takes CLOC12.148 to avoid collision
  when both merge.)

### gap-151 — `if`-both-branches-terminate not recognised as unreachable

Upstream removes code after an `if` whose every branch terminates
(`if (c) return; else return; y();` → the `y()` is unreachable). closurec's DCE
only truncates after a *direct* terminator statement; an `IfStatement` is not
`is_terminator`, so the trailing `y()` survives today. Placeholder:
`drops_code_after_if_both_branches_return`.

### gap-152 — `break`/`continue` only terminate switch cases, not loop blocks

Upstream treats `break`/`continue` as ending the enclosing loop body, so code
after a `break` inside a loop is unreachable (`while (c) { break; y(); }` →
`while (c) { break; }`). closurec only treats break/continue as terminators
inside switch-case consequents (`is_case_terminator`), so the loop-body `y()`
survives. Placeholder: `drops_code_after_break_in_loop_block`.

## CLOC12.149 — `FunctionExpression` (function in value position)

**Landed (AST + emitter + passes):** `Expression::FunctionExpression` is now a
first-class node (`javascript-ast`), the emitter prints it (statement-start and
call-callee parenthesisation, no trailing `;`), and every optimisation pass
(`constant-fold`, `fold-control-flow`, `dce`, `inline`, `inline-variables`,
`rename`, `rename-globals`, `rename-properties`, `scope-analyzer`) handles it
soundly — recursing into the body, with the function's own name/params removed
from the active rename/substitute map so a shadowed binding is never rewritten.
The expression sibling of `FunctionDeclaration`; `id` is `Option<Identifier>`
(anonymous, or a body-local name). Spec: CLOC09 §"Phase 1.x FunctionExpression".

### gap-153 — bridge declines `function_expression` — **RESOLVED** (javascript-parser 0.20.0)

The typed-AST bridge (`javascript-parser::bridge`) now converts
`function_expression` → `Expression::FunctionExpression` via
`convert_function_expression` (mirrors `convert_function_declaration`, name
optional), removed from the `UnsupportedSyntax` list. A function in value
position — IIFE `(function(){})()`, assigned `x = function(){}`, named recursive
`function f(){…f()…}`, callback `arr.map(function(x){…})` — flows through the full
pipeline; closurec optimises **inside** the body instead of falling back to
WHITESPACE_ONLY. Proven by the `tests/diff/simple-function-expression/` closurec
fixture (all four value positions fold their bodies at SIMPLE) plus 4 bridge
unit tests. Generators/async, arrow functions (`arrow_function`), classes, and
template literals remain declined — separate future AST slices.

## CLOC12.150 — CodePrinter function-expression conformance port

New upstream port `closure-emitter/tests/upstream/code_printer_function_test.rs`
(from `CodePrinterTest.java`), unblocked by CLOC12.149 (`FunctionExpression`
node + emitter) and gap-153 (bridge conversion). **12 active `#[test]`s, no
`#[ignore]`, no new gaps** — the emitter conforms to every covered
function-expression printing shape: anonymous/named, params, return body (no
trailing `;` after `}`), the three parenthesised contexts (statement-start,
IIFE callee `(function(){})()`, member object `(function(){}).x`), the
un-parenthesised call-argument context, and the `function*` / `async function`
prefixes. Getters/setters/methods (object-literal function values) and arrow
functions remain deferred to their own AST slices.

## CLOC12.151 — `ArrowFunctionExpression` (the `=>` form)

New `Expression::ArrowFunctionExpression { cv, params, body: ArrowBody, is_async }`
in `javascript-ast` (0.15.0), plus the emitter and every pass traversal — one
**atomic** PR, because adding an `Expression` variant makes every exhaustive
`match` across the workspace non-exhaustive, and CI builds the workspace.

Structural simplifications from `FunctionExpression`: no `id` (arrows are always
anonymous), no `generator`, and a dual-shape `body` — the new `ArrowBody` enum is
either a `Block(BlockStatement)` (`x => { return x; }`) or a concise
`Expression(Box<Expression>)` (`x => x`). Emitter: single plain-identifier param
drops parens (`x=>x`); object-literal concise bodies are wrapped (`()=>({a:1})`);
arrows tag at `PREC_ASSIGNMENT` so call-callee/member-object parents wrap them; no
statement-start wrap. Renaming/inlining passes drop the arrow's param names from
the active map before recursing so a shadowing param is left untouched.

Landed bottom-up: node + emitter + passes here; the parser→typed-AST bridge
(`javascript-parser` currently declines `arrow_function` as `UnsupportedSyntax`)
and an upstream conformance port follow in later slices.

### gap-154 — deeper leftmost-`{` concise arrow body not wrapped

The emitter wraps a concise body that is *directly* an `ObjectExpression`
(`()=>({a:1})`), but not one whose leftmost token is `{` through a larger
expression — e.g. `()=>({}).x` (a member access on an object literal) currently
emits `()=>{}.x`, which mis-parses the `{}` as a block body. This is the same
leftmost-token class as the general expression-statement `{`-wrap and is rare in
practice (an object-literal-rooted concise body used as a member/call base);
deferred until a general leftmost-token analysis lands.

## CLOC12.152 — bridge `arrow_function` → `Expression::ArrowFunctionExpression`

The `javascript-parser` typed-AST bridge (0.21.0) now converts **concise-body
arrow functions** (`x => x + 1`, `(a, b) => a + b`, `() => 1`,
`arr.map(x => x)`) — `convert_arrow_function` reads `arrow_parameters`
(`NAME` → one param; `( formal_parameters )` → the list; `()` → none) and the
`concise_body` expression, yielding an `ArrowFunctionExpression` with an
`ArrowBody::Expression`. This unblocks the SIMPLE pipeline *inside* arrow bodies
end-to-end (`var f = x => 1 + 2` → `var f = x => 3`) instead of the previous
whitespace-only fallback. 5 bridge tests + a closurec e2e diff fixture
(`tests/diff/simple-arrow-function/`).

### gap-155 — bridge declines `arrow_function` — **RESOLVED** (javascript-parser 0.21.0)

Previously the bridge listed `arrow_function` under `UnsupportedSyntax`. Now
resolved for concise-body arrows (see above). Async / block-bodied / object-body
arrows remain out of scope per gap-156 and the async note.

### gap-156 — grammar rejects block-bodied arrows; `() => {}` mis-parses as object body

Two *grammar*-level limitations (in `code/grammars/ecmascript/*.grammar`, not the
bridge) constrain CLOC12.152 to concise expression bodies:

1. **Block-bodied arrows fail to parse.** `x => { return x; }` — the
   `concise_body = … | LBRACE function_body RBRACE` alternative is not taken by
   the generated parser, so a statement block body is a hard parse error. The
   bridge's `ArrowBody::Block` path and the emitter/passes already model block
   bodies; only the parser needs fixing (a separate grammar PR).
2. **`() => {}` is read as an empty-*object* concise body.** With the block
   alternative unavailable, `{}` after `=>` parses as an empty object literal —
   i.e. `() => ({})` — which returns a *different value* (an object) than the
   real empty-block arrow (returns `undefined`). The distinguishing information
   is already lost in the parse tree, so the bridge **declines** any arrow whose
   concise body is an `ObjectExpression` (falls back to whitespace-only, which
   re-emits the source unchanged and is always correct). Genuine object-returning
   arrows (`x => ({a:1})`) decline too — forgoing an optimisation, never
   correctness. Fixing the grammar's block-body parse resolves both.

## CLOC12.153 — CodePrinter arrow-function conformance port (emitter)

New upstream port `closure-emitter/tests/upstream/code_printer_arrow_test.rs`
(from `CodePrinterTest.java`), isolating `emit_arrow_function_expression` + the
`PREC_ASSIGNMENT` wrap that landed with `Expression::ArrowFunctionExpression`
(CLOC12.151). **12 active `#[test]`s, no `#[ignore]`, no new gaps** — the emitter
conforms to every covered arrow printing shape: single-param param-paren drop
(`x=>x`), zero/multi param (`()=>1`, `(a,b)=>a`), object-literal-body wrap
(`()=>({})`), concise vs block body (`x=>{return x}`), IIFE / member-object wrap
(`(()=>{})()`, `(()=>{}).x`), un-parenthesised call-argument (`g(x=>x)`), and the
`async` prefix (`async x=>x`, `async()=>{}`). Inputs are hand-constructed AST, so
the port also covers **block-bodied** arrows that the current grammar can't yet
parse (see gap-156, added with CLOC12.152) — the emitter already prints them
correctly; only the parser needs the grammar fix.

## CLOC12.154 — `TemplateLiteral` (backtick template strings)

New `Expression::TemplateLiteral { cv, quasis: Vec<TemplateElement>,
expressions: Vec<Expression> }` + `TemplateElement { cv, raw, cooked:
Option<String>, tail }` in `javascript-ast` (0.16.0), plus the emitter and every
pass traversal — one **atomic** PR (adding an `Expression` variant makes every
exhaustive `match` across the workspace non-exhaustive; CI builds the workspace).

A template interleaves fixed string parts (`quasis`) with `${…}` `expressions`,
satisfying the ESTree invariant `quasis.len() == expressions.len() + 1`. Emit
prints `` `q0${e0}q1${e1}…qN` `` from each quasi's verbatim `raw` text; a template
tags at `PREC_PRIMARY` (never wrapped). Because a template introduces **no
bindings and no scopes**, every pass arm simply recurses into `expressions` (the
`quasis` are leaf strings) — no map reduction / shadowing logic, unlike the arrow
arms.

Landed bottom-up: node + emitter + passes here; the parser→typed-AST bridge
(`javascript-parser` currently declines `template_literal` as `UnsupportedSyntax`)
and an upstream conformance port follow in later slices. *Tagged* templates
(`` tag`…` ``, node `TaggedTemplateExpression`) remain Phase 3.

### Future — fold an all-constant template to a string literal

`` `a${1}b` `` could fold to the string literal `"a1b"` when every `${…}` is a
constant. Not done here — CLOC12.154 only recurses to fold *inside* each `${…}`;
whole-template folding is a later constant-fold enhancement.

## CLOC12.155 — bridge `template_literal` → `Expression::TemplateLiteral` (no-substitution)

The `javascript-parser` typed-AST bridge (0.22.0) now converts **no-substitution
template literals** (`` `abc` ``, `` `` ``) instead of declining them as
`UnsupportedSyntax`. `convert_template_literal` reads the single
`TEMPLATE_NO_SUB` token (see gap-157), strips its leading and trailing `` ` ``,
and yields a `TemplateLiteral` with one tail `TemplateElement { raw, cooked:
Some(raw), tail: true }` and empty `expressions`. This unblocks the full
SIMPLE/ADVANCED pipeline for files containing plain templates end-to-end — a file
like `` log(`hello`, 1 + 2) `` now folds to `` log(`hello`, 3) `` instead of the
whole file dropping to WHITESPACE_ONLY on the bridge decline. Bridge tests +
closurec e2e diff fixture (`tests/diff/simple-template-literal/`).

### gap-157 — grammar tokenises only *no-substitution* templates; `` `${…}` `` does not parse

The ECMAScript grammar (in `code/grammars/ecmascript/*.grammar`) lexes a
substitution-free backtick template as a **single** `TEMPLATE_NO_SUB` token whose
value is the entire literal, backticks included (`` `abc` `` → one token
`"`abc`"`). Templates *with* a `${…}` substitution — which require the lexer to
split `` `head${ ``, ` }middle${ `, ` }tail` ` into template-head / middle / tail
tokens interleaved with expression parses — do **not** parse at all today (a hard
parse error). So CLOC12.155 is scoped to the no-substitution case: the bridge's
`convert_template_literal` **declines** any `template_literal` node that is not
exactly one `TEMPLATE_NO_SUB` token (falling back to WHITESPACE_ONLY, always
correct). The `TemplateLiteral` AST node already models the multi-part shape
(`quasis` / `expressions`), so once the grammar learns to split substitution
templates the converter grows a multi-part branch with no node change. *Tagged*
templates (`` tag`…` ``) remain a separate Phase 3 slice.

### Future — cook escape sequences in `raw` → `cooked`

CLOC12.155 sets `cooked = Some(raw)` verbatim. A no-substitution template with an
escape (`` `a\tb` ``) should have a `cooked` value with the escape *processed*
(`a⇥b`) distinct from its `raw` text. The emitter re-emits `raw` regardless, so
this is never a correctness hazard for round-tripping; genuine cooked-value
processing is a later refinement (needed once a pass reads `cooked` to fold a
template to a string literal).

## CLOC12.156 — CodePrinter template-literal conformance port (emitter)

`closure-emitter` (0.21.2) gains `tests/upstream/code_printer_template_test.rs`
— the tenth CodePrinter port, isolating `emit_template_literal` /
`emit_template_element` + the `PREC_PRIMARY` classification that landed with
`Expression::TemplateLiteral` (CLOC12.154). 17 active `#[test]`s and 1
`#[ignore]`. Coverage: no-substitution templates (incl. empty, escaped
backtick, escaped `${`), a template as an unwrapped member-access object and
as an unwrapped `+` / `==` operand (it is primary), and `${…}` substitution
templates (single, adjacent, text-interleaved, low-precedence body,
member-access body). Inputs are hand-constructed AST so the port exercises
`${…}` substitution templates the grammar tokenises only as no-substitution
today (gap-157) — the emitter already prints them; the parser can't yet feed
them. Test-only PR (no emitter code change); the one ignored case surfaces
gap-158. *Tagged* templates are intentionally not ported (no
`TaggedTemplateExpression` AST node — see the port's ATTRIBUTION).

### gap-158 — emitter cannot print a template quasi with a *literal* embedded newline — **RESOLVED (closure-emitter 0.21.3, CLOC12.157)**

`emit_template_element` wrote a quasi's `raw` text through the emitter's
`write_str`, which `debug_assert!`s the run contains no `'\n'` — every newline
must instead flow through `newline()` so the source-map line/column bookkeeping
(and `line`/`col` counters) stay correct. A template quasi is the one *primary*
token that can legitimately carry a raw newline (a multiline template such as
`` `hello⏎world` `` preserves it verbatim per spec), so a quasi whose `raw`
contained `\n` panicked the emitter worker rather than printing.

**Fix (CLOC12.157):** `emit_template_element` now splits the quasi `raw` on
`'\n'` and writes each line segment via `write_str` with a real `newline()`
between segments — so `line` advances and `col` resets exactly as for any other
newline, and the raw bytes round-trip. `str::split('\n')` yields `N + 1` pieces
for `N` newlines (empty leading/trailing pieces for a raw that starts/ends with
`'\n'` write nothing), so the line count lands exactly on the `'\n'` count.
Single-line quasis are one segment → identical to before. The conformance
port's `raw_preserves_internal_newline` is un-ignored and a
`raw_preserves_leading_and_trailing_newline` case plus three inline emitter
tests were added.

**Residual (not a hazard):** other line-terminator bytes a raw may carry — a
lone `'\r'` (the `'\r'` of a `\r\n` pair rides on the tail of the preceding
segment) and the Unicode separators `U+2028` / `U+2029` — are written verbatim
as ordinary characters: bytes round-trip exactly, and only their *column*
bookkeeping is approximate (they don't increment `line`). That is a source-map
nicety, not an output-correctness bug; a fully line-terminator-aware split is a
later refinement if a source-map consumer ever needs column fidelity across
CR / LS / PS inside a template.

## CLOC12.157 — newline-aware template quasi emit (resolves gap-158)

`closure-emitter` (0.21.3) makes `emit_template_element` newline-aware: a
multiline template quasi (`raw` containing a literal `'\n'`) now prints its
interior line break verbatim instead of panicking the emitter worker on
`write_str`'s no-embedded-newline `debug_assert!`. See gap-158 (now RESOLVED)
for the mechanism. First non-test-only change in the TemplateLiteral arc since
CLOC12.154 — a real `src/` fix in the emitter. The CLOC12.156 conformance
port's `raw_preserves_internal_newline` case is un-ignored (now 19 active
`#[test]`s, 0 `#[ignore]` — the port fully conforms), and three inline emitter
unit tests cover the no-sub, substitution-quasi, and bare-newline shapes. This
completes multiline handling for the template shapes the emitter can be handed;
the grammar still only tokenises no-substitution templates into the bridge
(gap-157), so multiline templates reach the emitter today only via
hand-constructed AST, but the emitter is now correct for when the bridge grows
multiline support.

## CLOC12.158 — `UpdateExpression` (`++` / `--`): atomic node + emit + passes (PR1)

Adds `Expression::UpdateExpression` (`UpdateExpression { operator, prefix,
argument }`) + the `UpdateOperator` enum (`++` / `--`) to `javascript-ast`
(0.17.0) — the first fundamental non-template `Expression` node added since the
arrow arc, and the last "Phase 2" expression form the 0.16.0 `UnaryExpression`
docs still deferred. Kept **distinct** from `UnaryExpression`: `++`/`--` are a
read-modify-write, so they (a) carry a side effect (DCE/purity must not drop
`x++`) and (b) require a *writable reference* operand (identifier / member),
never an arbitrary value. `prefix` splits `++x` (yield the new value) from
`x++` (yield the old value).

**Atomic scope (PR1):** the node, the emitter (`emit_update`, `closure-emitter`
0.22.0), and every exhaustive `Expression` match across the pass crates land in
one change so the workspace never has a broken build — same shape as the
ArrowFunctionExpression (CLOC12.151) and TemplateLiteral (CLOC12.154) PR1s.
Downstream crates gained a match arm (patch bumps): constant-fold, dce,
fold-control-flow, inline, inline-variables, rename, rename-globals,
rename-properties, scope-analyzer. Emitter handling: `PREC_UNARY` precedence
(wraps an `**` base and a member/call object, prints bare under `!`/`typeof`)
and token-fusion seams via `arg_starts_with_sign` reporting a prefix update's
lead sign (`a- --b`, `a+ ++b`) plus the binary emitter's output-tail check for
postfix (`x++ +y`). 8 emitter unit tests.

### gap-159 — bridge declines `update_expression` — **RESOLVED (javascript-parser 0.23.0, CLOC12.158 PR2)**

The `javascript-parser` typed-AST bridge returned `UnsupportedSyntax` for
`++`/`--` (both the grammar's `postfix_expression = lhs [ ++ | -- ]` and the
prefix `++x`/`--x` unary alternative) — the decline predated the
`UpdateExpression` node and was correct while the typed AST had no such node.

**Fix (CLOC12.158 PR2):** with the node in place (PR1), the bridge now
converts it — `convert_postfix_expression` builds a postfix
`UpdateExpression { prefix: false }`, the prefix `++`/`--` arm of
`convert_unary_expression` builds `{ prefix: true }`, and the shared helper
`update_operator_from_node` maps the `++`/`--` token child. The operand is
converted as a normal expression. The whole SIMPLE/ADVANCED pipeline now runs
on files containing `++`/`--` end-to-end instead of dropping to
WHITESPACE_ONLY. 6 bridge tests + a closurec e2e diff fixture
(`tests/diff/simple-update-expression/`) proving statement `i++` round-trips
(never dropped to `i`) while an adjacent `1 + 2` folds to `3`. The
additive-with-unary-sign invariant (`a + +b`, `a - -b` stay binary, never
mis-read as `++`) is pinned by a bridge test.

The **PR3 emit conformance port** (closure-emitter 0.22.1, CLOC12.158 PR3)
ports upstream CodePrinter's update-operator cases into
`closure-emitter/tests/upstream/code_printer_update_test.rs` (registered as
the `upstream_code_printer_update` `[[test]]` target): prefix/postfix
increment/decrement, a member operand (`a.b++`), bare printing under `!` /
`typeof`, the `PREC_UNARY` precedence wraps (`(x++).y`, `(++x)**2`), and the
`+`/`-` token-fusion seams (`a- --b`, `a+ ++b`, `x++ +y`, `x-- -y`, plus the
non-fusing `x++*y`). 14 active `#[test]`s, no `#[ignore]` — the emitter
conforms to every covered shape. This completes the CLOC12.158
`UpdateExpression` three-PR arc (node+emit → bridge → conformance).

## CLOC12.159 — `NewExpression` (`new X(args)`): atomic node + emit + passes (PR1)

Adds `Expression::NewExpression { cv, callee, arguments }` to `javascript-ast`
(0.18.0) — the `new` operator, `new Ctor(a, b)`. Structurally a
`CallExpression` (callee + argument list) but a **separate** node: its
evaluation semantics are object *construction* (a pass must never rewrite
`new f()` into `f()`), and its grammar precedence differs (§ below).

`closure-emitter` (0.23.0) `emit_new` prints `new`, the callee, then the
argument parens. The parens are **always printed**, so a no-argument node is
emitted canonically as `new X()` (never bare `new X`) — this keeps every
emitted `new` in the *argumented* form, which binds at member/call strength
(`PREC_PRIMARY`), so `new X().y` needs no extra parens. Two seams:

  * **keyword space** — `new`↔identifier/member callee is a word↔word boundary
    (`newX` would fuse), spent via `required_ws` unless the callee is already
    parenthesised;
  * **callee-with-call wrap** — the `new` target is a `MemberExpression` per
    grammar and cannot itself be a call, so a callee whose member spine bottoms
    out in a call is wrapped: `new (f())()`, `new (a.b().c)()` — otherwise the
    appended `(args)` binds to the inner call (`new f()()` = `(new f())()`, a
    different program). `new_callee_needs_parens` decides this.

All nine downstream pass/analysis crates (`constant-fold`, `dce`,
`fold-control-flow`, `inline`, `inline-variables`, `rename`, `rename-globals`,
`rename-properties`, `scope-analyzer`) get a `NewExpression` match arm
mirroring `CallExpression` (recurse callee + args), landed in the same atomic
commit so the workspace never has a broken exhaustive `match`. The `inline`
pass explicitly does **not** treat a `new` as an inlinable call (it is a
construction), so its tally arm recurses without the call-inlining gate.

8 new emitter unit tests (identifier/args/member-chain callee, call-callee and
member-spine-call wraps, argumented-new as member object, no-arg canonical
`new X()`, nested `new new X()()`). No behaviour change for existing inputs —
the bridge still declines `new` (gap-160), so closurec falls back to
WHITESPACE_ONLY on `new` for now. PR2 (bridge-enable) and PR3 (conformance
port) follow.

### gap-160 — bridge declines `new_expression` (`new X`) — **RESOLVED (javascript-parser 0.24.0, CLOC12.159 PR2)**

The `javascript-parser` typed-AST bridge declined the `new` operator to
`UnsupportedSyntax`, so any file containing `new` dropped to WHITESPACE_ONLY.

**Fix (CLOC12.159 PR2):** with the `Expression::NewExpression` node in place
(PR1), the bridge now converts both grammar productions in which `new` appears:

- **argumented** `new X(args)` parses as `member_expression = "new"
  member_expression arguments` — `convert_member_expression` builds a
  `NewExpression { callee, arguments }` (arguments via `convert_arguments`) as
  the base and folds any trailing `.NAME` / `[expr]` suffix onto it, so
  `new X().y` / `new X()[k]` convert correctly;
- **bare** `new X` parses as `new_expression = "new" new_expression` —
  `convert_new_expression` builds a `NewExpression` with an EMPTY argument list
  (semantically identical to `new X()`; the emitter prints the canonical
  `new X()`).

A spread argument (`new X(...a)`) still declines (`SpreadElement`, later slice)
and `new.target` still declines (`NewTarget`, Phase 3). 7 bridge tests + a
closurec e2e diff fixture (`tests/diff/simple-new-expression/`) proving
`log(new Widget(1 + 2))` round-trips the construction while `1 + 2` folds to
`3` (a WHITESPACE_ONLY fallback would leave it unfolded).

The **PR3 emit conformance port** (closure-emitter 0.23.1, CLOC12.159 PR3)
ports upstream CodePrinter's `new`-operator cases into
`closure-emitter/tests/upstream/code_printer_new_test.rs` (registered as the
`upstream_code_printer_new` `[[test]]` target): identifier / member-chain
callees (`new X()`, `new a.b.c()`), argument lists (`new X(a,b)`, `new X(a.b)`),
the callee-with-call wraps (`new (f())()`, `new (a.b().c)()`), the
`PREC_PRIMARY` member-object cases (`new X(a).y`, `new X().y`), nested
`new new X()()`, and a call on a `new` member (`new X().m()`). 10 active
`#[test]`s, no `#[ignore]` — the emitter conforms to every covered shape. This
completes the CLOC12.159 `NewExpression` three-PR arc (node+emit → bridge →
conformance).

## CLOC12.160 — `SequenceExpression` (`a, b, c`): atomic node + emit + passes (PR1)

Adds `Expression::SequenceExpression { cv, expressions }` to `javascript-ast`
(0.19.0) — the **comma operator**: evaluate each operand left to right, yield
the last. It is the **loosest** expression in the grammar (the entry production
`Expression : AssignmentExpression { , AssignmentExpression }`), binding below
assignment.

`closure-emitter` (0.24.0) `emit_sequence` prints the operands comma-joined
(`a,b,c`), each at `PREC_ASSIGNMENT`. The sequence itself is the new lowest
precedence `PREC_SEQUENCE` (0). Correct wrapping of a sequence sub-operand
required the four **assignment-position** emit sites to emit their child at
`PREC_ASSIGNMENT` rather than the parent-precedence-0 sentinel: **call
arguments, `new` arguments, array elements, and the assignment RHS**. A sequence
there now wraps — `f((a,b),c)` (never the three-argument `f(a,b,c)`),
`[(a,b),c]` (never the three-element `[a,b,c]`), `x=(a,b)` (never `x=a,b` =
`(x=a),b`). This is a **no-op for every existing node type** (all bind at
`PREC_ASSIGNMENT` or higher, so nothing new wraps — the full closurec suite is
unchanged). Two positions keep a bare sequence: statement position (`a,b,c;`)
and a computed-member key (`obj[a,b]`, which the surrounding `[ ]` already
delimits).

All nine downstream pass/analysis crates get a `SequenceExpression` match arm
recursing into `expressions` (the transforms rebuild it; `constant-fold` folds
each operand but keeps them all — earlier operands may have side effects), and
`fold-control-flow`'s exhaustive `expression_cv` accessor gains its arm — all in
one atomic commit so the workspace never has a broken exhaustive `match`.

8 new emitter unit tests (statement-bare, sole/multi call arg, array element,
assignment RHS, computed-member-key-bare, conditional branch, unary operand).
No behaviour change for existing inputs — the bridge still declines the comma
operator (gap-161), so closurec falls back to WHITESPACE_ONLY on `a, b, c` for
now. PR2 (bridge-enable) and PR3 (conformance port) follow.

### gap-161 — bridge declines the comma operator (`SequenceExpression`) — **RESOLVED (javascript-parser 0.25.0, CLOC12.160 PR2)**

The `javascript-parser` bridge's `convert_expression_rule` handled only the
single-expression form of `expression = assignment_expression { COMMA
assignment_expression }`; a multi-operand comma list declined to
`UnsupportedSyntax`, dropping the whole file to WHITESPACE_ONLY.

**Fix (CLOC12.160 PR2):** with the `Expression::SequenceExpression` node in
place (PR1), `convert_expression_rule` now builds it from the operand list.
`node_children` already drops the `COMMA` tokens, so the `assignment_expression`
children convert directly into `expressions`, in source order; the
single-operand path is unchanged (it passes the operand through, never wrapping
a one-element sequence). Wherever the grammar's `expression` rule appears —
statement position (`a, b, c;`), a parenthesised group (`x = (a, b)`), a
computed-member key (`obj[a, b]`) — the comma operator now flows through the
full SIMPLE/ADVANCED pipeline end-to-end. 4 bridge tests + a closurec e2e diff
fixture (`tests/diff/simple-sequence-expression/`) proving `log((a, 1 + 2))`
round-trips the sequence parenthesised while `1 + 2` folds to `3`.

The **PR3 emit conformance port** — the remaining follow-up — ports upstream
CodePrinter's comma-operator cases into `closure-emitter/tests/upstream/`.

## CLOC12.161 — `TaggedTemplateExpression` (`` tag`...` ``): atomic node + emit + passes (PR1)

Adds `Expression::TaggedTemplateExpression { cv, tag, quasi }` to `javascript-ast`
(0.20.0) — a **tagged template**: the `tag` callee is applied to the template
literal that directly follows it, `` tag`abc${x}` ``, with **no call
parentheses**. The `quasi` field is a `TemplateLiteral` (the CLOC12.154 node),
so no new template shape is introduced — a tagged template is "an expression
applied to a template", nothing more at the AST level. (The runtime semantics —
the tag receives the cooked/raw strings array plus the substitution values — are
a call-site concern the minifier never evaluates; structurally it is just the
two children.)

`closure-emitter` (0.25.0) `emit_tagged_template` prints the `tag` at
`PREC_PRIMARY` immediately followed by `emit_template_literal(&quasi)` — the
backtick abuts the tag with no separator seam (`` String.raw`x` ``). The node is
itself `PREC_PRIMARY` in `expr_prec`, so a member access on a tagged template is
paren-free (`` a`x`.length ``); a **looser** tag is wrapped by the existing
`PREC_PRIMARY` object-position rule (a sequence tag becomes `` (a,b)`x` `` so it
does not tag only its last operand). No new emit-site precedence surgery was
needed — the `PREC_PRIMARY` classification and the tag callee emitting at
`PREC_PRIMARY` reuse the machinery the member/call nodes already established.

All nine downstream pass/analysis crates get a `TaggedTemplateExpression` match
arm recursing into the `tag` callee **and** each `${…}` insert of `quasi`
(the transforms rebuild the node; `fold-control-flow`'s exhaustive
`expression_cv` accessor gains its arm) — all in one atomic commit so the
workspace never has a broken exhaustive `match`.

5 new emitter unit tests (identifier-tag no-substitution, member-chain tag,
`String.raw` tag with a `${}` substitution, member access on a tagged template,
sequence tag wrapped). No behaviour change for existing inputs — the bridge
still declines the tagged-template form (gap-162), so closurec falls back to
WHITESPACE_ONLY on `` tag`...` `` for now. PR2 (bridge-enable) and PR3
(conformance port) follow.

### gap-162 — bridge declines tagged templates (`TaggedTemplateExpression`) — **RESOLVED (javascript-parser 0.26.0, CLOC12.161 PR2)**

The `javascript-parser` bridge declined a `template_literal` node appearing as a
suffix on a base expression (`<tag> <template_literal>`), returning
`UnsupportedSyntax` and dropping the whole file to WHITESPACE_ONLY. The atomic
node PR (CLOC12.161 PR1, #7495) landed the node + emit + pass traversals.

**Fix (CLOC12.161 PR2):** `convert_member_expression`'s suffix walk now converts
a trailing `template_literal` into `Expression::TaggedTemplateExpression` — the
accumulated `base` becomes the `tag`, and the template becomes the `quasi` via
the existing `convert_template_literal` (CLOC12.155, reused unchanged). Because
the wrap continues the suffix walk, chained forms like `` a`x`.length `` and
`` a`x`() `` flow through naturally. 4 bridge tests (identifier tag, member-chain
tag, member-access-on-tagged chaining, and a guard that an *untagged* template
still bridges to a bare `TemplateLiteral`), plus a closurec e2e diff fixture
(`tests/diff/simple-tagged-template/`) proving `log(tag`abc`, 1 + 2)` →
`log(tag`abc`,3)`: the tag round-trips while the sibling `1 + 2` folds to `3`,
proving the whole file ran through SIMPLE rather than WHITESPACE_ONLY.

**Scope:** substitution templates `` `a${x}b` `` still do not parse in the
grammar (`convert_template_literal` handles the single-`TEMPLATE_NO_SUB`-token
form only), so the tagged form is enabled **no-substitution** — matching the
template bridge's scope. When the grammar learns `${…}`, both the plain and
tagged template converters grow the substitution branch together; the AST node
already models it.

## CLOC12.162 — `SpreadElement` (`...arg`): atomic node + emit + passes (PR1)

Adds `Expression::SpreadElement { cv, argument: Box<Expression> }` to
`javascript-ast` (0.21.0) — the JS **spread** `...arg`: the `...` prefix that
unpacks an iterable into a call/`new` argument list (`f(...a)`, `new F(...a)`)
or an array-literal element (`[1, ...rest, 2]`). Unlike every other
`Expression` variant a spread is **not a free-standing expression** — bare
`...x` is a syntax error, and the grammar only admits it in argument/element
position. It is nonetheless modelled as an `Expression` variant so it slots
directly into the `Vec<Expression>` argument/element lists that
`CallExpression`, `NewExpression`, and `ArrayExpression` already carry: no
parallel "argument-or-spread" enum is introduced, and every AST walker that
already recurses those `Vec`s reaches the spread's inner `argument` for free.
(Object-spread `{...o}` is a *property*, a separate later slice; this node is
the call/array/`new` spread.)

`closure-emitter` (0.26.0) `emit_spread` prints the three literal `.`
characters then the `argument` at `PREC_ASSIGNMENT`, with no interior space
(`...a`, never `... a`). That precedence is the crux: the argument grammar is
an `AssignmentExpression`, so everything at or above assignment strength prints
bare (`...a`, `...a.b`, `...f()`, `...a?b:c`, `...a=b`), while the one looser
form — a **sequence** — is wrapped (`...(a,b)`), because a bare `...a,b` would
spread only `a` and leave `,b` as a second list slot (a miscompile). The node
itself tags at `PREC_ASSIGNMENT` in `expr_prec`, matching the
assignment-position list slots it lives in (`f(...a)`, `[...a]`) so it is never
spuriously parenthesised there; it is never emitted as a sub-operand of another
operator (that would be invalid JS), so no other context observes the tag. No
new emit-site precedence surgery was needed.

All nine downstream pass/analysis crates get a `SpreadElement` match arm
recursing into `argument` (the transforms rebuild the node; `fold-control-flow`'s
exhaustive `expression_cv` accessor gains its arm) — all in one atomic commit so
the workspace never has a broken exhaustive `match`.

6 new emitter unit tests (sole call arg `f(...a)`, interleaved
`f(a,...b,c)`, array element `[1,...a,2]`, `new` arg `new F(...a)`, the
sequence-arg wrap `f(...(a,b))`, and the conditional-arg bare `f(...a?b:c)`
showing no over-wrap). No behaviour change for existing inputs — the bridge
still declines spread (gap-163), so closurec falls back to WHITESPACE_ONLY on
`...` for now. PR2 (bridge-enable) and PR3 (conformance port) follow.

### gap-163 — bridge declines spread (`SpreadElement`) — **RESOLVED (javascript-parser 0.27.0, CLOC12.162 PR2)**

The `javascript-parser` bridge declined a `...` spread appearing in a call/`new`
argument list or an array element, returning `UnsupportedSyntax` and dropping
the whole file to WHITESPACE_ONLY. The atomic node PR (CLOC12.162 PR1, #7515)
landed the node + emit + pass traversals so the typed AST and every downstream
pass can already represent and print a spread.

**Fix (CLOC12.162 PR2):** dumping the parse tree showed spread parses to a
dedicated `spread_element` node whose children are
`[ Token("..."), Node(assignment_expression) ]`, and that node appears as a
`Node` child of both `argument_list` (calls / `new`) and `element_list`
(arrays). `convert_argument` and `convert_array_literal`'s `has_token(_, "...")`
branch now convert the inner expression (via `node_children`, which strips the
ELLIPSIS token) and wrap it as `Expression::SpreadElement` rather than
declining. 6 bridge unit tests (call spread, interleaved-call arity, `new`
spread, array spread, interleaved-array count, and a non-spread guard), plus a
closurec e2e diff fixture (`tests/diff/simple-spread/`) proving
`log(...a, 1 + 2)` → `log(...a,3)`: the spread round-trips while the sibling
`1 + 2` folds to `3`, proving the whole file ran through SIMPLE rather than
WHITESPACE_ONLY.

**Scope:** object-spread `{...o}` in an object literal (`convert_property_definition`,
gap tracked as `SpreadProperty`) is a separate *property*-position slice, taken up
in **CLOC12.170** (its node + emit landed in PR1; bridge in PR2). This PR is the
call/array/`new` spread only.

## CLOC12.163 — `YieldExpression` (`yield` / `yield*`): atomic node + emit + passes (PR1)

Adds `Expression::YieldExpression { cv, delegate: bool, argument: Option<Box<Expression>> }`
to `javascript-ast` (0.22.0) — the generator **yield** operator that pauses a
generator and surrenders a value to its caller. Two independent axes are kept
as separate fields so the emitter and every AST walker reason about them
independently:

| source        | `delegate` | `argument` |
|---------------|------------|------------|
| `yield`       | `false`    | `None`     |
| `yield x`     | `false`    | `Some(x)`  |
| `yield* xs`   | `true`     | `Some(xs)` |

`delegate` is the `*` of a *delegating* yield (which forwards every value its
argument iterable produces); `argument` is `None` only for a bare `yield`. The
argument is boxed so the `Expression` enum stays a fixed size. The variant is
re-exported from the crate root alongside the other expression nodes.

`closure-emitter` (0.27.0) `emit_yield` prints the `yield` keyword, then the `*`
for a delegate, then — if an argument is present — the argument at
`PREC_ASSIGNMENT`. **Token separation:** a non-delegating `yield` needs a
mandatory `required_ws()` before its argument (`yield x`; `yieldx` would be one
identifier), while after a delegating `yield*` the `*` already terminates the
keyword token so the argument abuts it (`yield*xs`). **Precedence:** `yield` is
an `AssignmentExpression` alternative — the loosest-binding expression bar the
comma — so a conditional or assignment argument prints bare
(`yield a?b:c`, `yield a=b`) while the one looser form, a **sequence**, wraps
(`yield (a,b)`). `expr_prec` tags the whole node at `PREC_ASSIGNMENT` (exactly
like arrow / assignment), so a tighter parent wraps it — `(yield a)+1`,
`(yield a).b` — with no local paren logic in `emit_yield`.

All nine downstream pass/analysis crates get a `YieldExpression` match arm that
recurses into the optional `argument` (the transforms — constant-fold, dce,
fold-control-flow — rebuild the node preserving `delegate` and mapping over the
`Option`; `fold-control-flow`'s exhaustive `expression_cv` accessor gains its
arm; the traversal passes — inline, inline-variables, rename, rename-globals,
rename-properties, scope-analyzer — walk into the argument when present) — all
in one atomic commit so the workspace never has a broken exhaustive `match`.

9 new emitter unit tests: bare `yield`, `yield x` (required space), `yield*xs`
(no space), `yield*a.b` (delegate member arg), the assignment-precedence
argument cases (`yield a?b:c` and `yield a=b` bare, `yield (a,b)` sequence
wrapped), and the whole-node wraps in tighter parents (`(yield a)+1`,
`(yield a).b`). No behaviour change for existing inputs — the bridge still
declines yield (gap-164), so closurec falls back to WHITESPACE_ONLY on generator
bodies for now. PR2 (bridge-enable) and PR3 (conformance port) follow.

### gap-164 — bridge declines `yield` (`YieldExpression`) — **RESOLVED (javascript-parser 0.28.0, CLOC12.163 PR2)**

The `javascript-parser` bridge declined a `yield` / `yield*` expression (and the
enclosing generator function), returning `UnsupportedSyntax` and dropping the
whole file to WHITESPACE_ONLY. The atomic node PR (CLOC12.163 PR1) landed the
node + emit + pass traversals so the typed AST and every downstream pass could
already represent and print a yield.

**Parseability check (the gate).** Unlike the preceding expression nodes,
`yield` is only grammatical inside a generator function body
(`function* g(){ … }`). Dumping the grammar parser's output confirmed:
`function*g(){}` parses (`generator_declaration`), and `function*g(){yield x;}`
/ `function*g(){yield* xs;}` parse with a **reachable `yield_expression`
node** — shape `[ Token("yield"), Node(assignment_expression) ]`, and delegate
`[ Token("yield"), Token("*"), Node(assignment_expression) ]`. So both operand
forms are parseable.

**Fix (CLOC12.163 PR2):** two joined bridge changes, because a bare
`yield_expression` conversion alone would still fall back to WHITESPACE_ONLY —
the *enclosing* `generator_declaration` / `generator_expression` was itself
declined. So this slice bridges both:
1. `convert_source_element` and `convert_expression` route
   `generator_declaration` / `generator_expression` through the existing
   function converters; a `*` token sets the `generator` flag (skipped during
   name extraction). The emitter already re-prints `function*`
   (`emit_function_declaration` / `emit_function_expression`), so no emit change
   was needed.
2. `convert_yield_expression` wraps the node as `Expression::YieldExpression`
   (`delegate = has_token(node, "*")`, operand = the sole child node via
   `node_children`).

5 bridge unit tests (generator declaration with `yield`, delegating `yield*`,
binary yield operand, generator expression in value position, plain-function
non-generator guard), plus a closurec e2e diff fixture
(`tests/diff/simple-yield/`) proving `use(function*(){yield 1 + 2;})` →
`use(function*(){yield 3})` at SIMPLE: the generator prints `function*`, the
`yield` round-trips, and the operand `1 + 2` folds to `3` — proving the whole
file ran through the pipeline rather than WHITESPACE_ONLY.

**Known grammar limitation (separate gap).** A bare operand-less `yield`
(`function*g(){yield;}`) does **not** parse today — the grammar's
`yield_expression` production requires an `assignment_expression` operand.
`Expression::YieldExpression` models `argument` as `Option` (a bare `yield` is
legal ES), but the bridge only ever produces `Some(_)` until the grammar admits
the operand-less form; `convert_yield_expression` surfaces an internal error
rather than mis-converting if a future operand-less node ever appears. Tracked
as a grammar gap, distinct from this structured-AST slice. (The existing
WHITESPACE_ONLY path also strips redundant parens around a `yield` operand,
gap-105 — a token-level fallback independent of this node.)

## CLOC12.164 — `AwaitExpression` (`await x`): atomic node + emit + passes (PR1)

Adds `Expression::AwaitExpression { cv, argument: Box<Expression> }` to
`javascript-ast` (0.23.0) — the **async-suspend** operator `await x`, which
pauses an async function until its operand promise settles and resumes with the
settled value. It is the async-side sibling of the generator `yield`
(CLOC12.163): both are context-restricted suspend operators. Two shape
differences from yield make the node *simpler*:

- **`argument` is a plain `Box<Expression>`, not `Option`.** A bare `await`
  (no operand) is a syntax error in every context, so the operand is always
  present — no `Option` is needed (contrast `yield`, where a bare `yield` is
  legal).
- **No `delegate` axis.** There is no `await*` form, so the node needs no
  second boolean.

`closure-emitter` (0.28.0) `emit_await` prints `await` then a **mandatory**
separator (`required_ws`) then the operand at `PREC_UNARY` — exactly the
treatment `emit_unary` gives the word-shaped unaries `typeof` / `void` /
`delete`. `await` is grammatically a unary operator (`await UnaryExpression`),
so it binds tighter than every binary operator and looser than member/call:

```text
  await a.b      → await a.b      member operand binds tighter → bare
  await f()      → await f()      call operand binds tighter   → bare
  await (a+b)    → await (a+b)    binary operand binds LOOSER   → wraps
  await a+b      → await a+b      the whole await binds tighter than + → (await a)+b bare
  (await p).x    → (await p).x    member parent binds at primary → wraps the await
  (await f)()    → (await f)()    call callee wraps the await
```

`expr_prec` tags `AwaitExpression` at `PREC_UNARY` (like the `void`/`typeof`
unary-word cases), which drives the member/call/new wrapping automatically —
no emit-site paren surgery. All nine downstream pass/analysis crates get an
`AwaitExpression` match arm recursing into `argument` (transforms rebuild the
node; `fold-control-flow`'s exhaustive `expression_cv` accessor gains its arm;
traversal passes walk into the argument) — structurally identical to the
`SpreadElement` arms since `argument` is a plain `Box`. All in one atomic commit
so the workspace never has a broken exhaustive `match`.

8 new emitter unit tests: mandatory space (`await p`), member/call operands
bare (`await a.b`, `await f()`), binary operand wrapped (`await (a+b)`), await
binding tighter than a binary parent (`await a+b`), member/call parents wrapping
the whole await (`(await p).x`, `(await f)()`), and a nested `await await p`. No
behaviour change for existing inputs — the bridge still declines `await`
(gap-165), so closurec falls back to WHITESPACE_ONLY on async bodies for now.
PR2 (bridge-enable) and PR3 (conformance port) follow.

### gap-165 — bridge declines `await` (`AwaitExpression`) — **OPEN**

The `javascript-parser` bridge does not yet convert an `await` expression to
`Expression::AwaitExpression`; an async function body containing an `await`
drops the whole file to WHITESPACE_ONLY. The atomic node PR (CLOC12.164 PR1)
landed the node + emit + pass traversals so the typed AST and every downstream
pass can already represent and print an await.

**PR2 (bridge-enable) — BLOCKED on grammar work (unlike the generator/yield
slice).** The parse-tree dump — the same gate that unblocked gap-164 for
generators — came back **negative for await**:

- `async function f(){}` **does** parse → an `async_function_declaration` node
  (with an `async` token), so the async-function *shell* is recognised.
- `async function f(){await p;}` **does NOT parse**: the parser reports an error
  at `p`, because inside the function body it treats `await` as a plain
  *identifier* (a primary expression) and then finds `p` where it expects an
  operator. The `await_expression` grammar production exists (it is a named rule
  in the decline list) but is **never reached** — the parser does not switch
  into an "async context" in which `await` becomes a prefix operator. Same for
  `async ()=>await p` and `x = async function(){await 1+2}`.

So, unlike `yield` (which parsed cleanly inside a generator body), `await` is
**not yet parseable**. Bridging it end-to-end requires *grammar* work — making
`await` a context-sensitive operator recognised inside async function /
async-arrow bodies — in the shared `code/grammars/` parser, a
larger and higher-risk change than a bridge slice (it is regenerated and shared
across every language that uses the grammar-driven parser). That grammar work is
deferred; PR2 waits on it.

Until then, PR1's node + the **PR3 conformance port**
(`code_printer_await_test.rs`) exercise the emitter via hand-constructed AST —
the same staging used for the substitution-template slice (gap-157), which also
shipped node + emit + conformance-port ahead of the parser. `emit_await` is thus
fully covered; only the parser→typed-AST reachability remains, tracked here.


## CLOC12.175 — class fields (`PropertyDefinition`): the first non-method class member (design)

CLOC12.173/174 shipped the class **expression** and **declaration** with a body of
**methods only** (`ClassMember::Method`). Both arcs' closing notes named the same
deferral: *instance/static **fields** (`PropertyDefinition`)… arrive in later
feature PRs, each an additive `ClassMember` variant.* CLOC12.175 is that slice —
the **first non-method class member**, and the first time `ClassMember` becomes a
multi-variant enum.

A class field is `x = 1;` / `y;` / `static z = 2;` / `[k] = v;` inside a class
body — a named slot with an optional initializer, no parameter list, no body.

### Model (ESTree-aligned, Rhino-informed)

```rust
// javascript-ast/src/expression.rs — a new variant of the `ClassMember` enum
pub enum ClassMember {
    Method(MethodDefinition),
    Field(PropertyDefinition),                 // NEW
    // static { … } (static-block) is a later additive variant.
}

pub struct PropertyDefinition {
    pub cv: Option<CvId>,
    pub key: PropertyKey,          // reuse — same four shapes as a method key
    pub value: Option<Expression>, // `x = <expr>` (Some) vs bare `x;` (None)
    pub computed: bool,            // `[expr] = v` (mirrors MethodDefinition.computed)
    #[serde(rename = "static")]
    pub is_static: bool,           // `static x = 1;`
}
```

**Why a separate `PropertyDefinition`, not reuse `MethodDefinition`.** A field has
**no** `MethodKind` (it is never a constructor/getter/setter), **no** function
`value` (its value is an *optional plain expression*, not a `FunctionExpression`),
and no params/body. This matches the conformance target — Closure's Rhino uses a
dedicated `MEMBER_FIELD_DEF` node, distinct from `MEMBER_FUNCTION_DEF`. `PropertyKey`
(identifier / string / numeric / `Expression` for computed) *is* reused — a field
key and a method key have the same four shapes.

### Emit (`closure-emitter`)

Inside a class body a field prints `[static ]key[=value];`:

```text
  class C { x = 1 }        → class C{x=1;}
  class C { y }            → class C{y;}
  class C { static z = 2 } → class C{static z=2;}
  class C { [k] = v }      → class C{[k]=v;}
```

Two differences from a method member: a field **ends with `;`** (a method's `}`
is self-terminating; a field has no closing brace so it needs the separator), and
it has **no** parameter-list/body tail. The `static` prefix and computed-key
bracketing reuse the existing `emit_class_member` machinery; the value (when
present) emits at `PREC_ASSIGNMENT` (the RHS of `=`), exactly like an
object-property value or a default. Fields and methods interleave in a body with
no separators beyond the field's own `;`.

### Pass arms — the `ClassMember` blast radius

Today `ClassMember` is single-variant, so nearly every pass matches it as
`match member { ClassMember::Method(m) => … }` or `let ClassMember::Method(m) =
member;`. Adding `Field` makes:

- every `match` **non-exhaustive** → the compiler forces a `Field` arm at each
  site (the per-crate build-tool is the ground truth for which, per the 174
  lesson — a compile error, not a green `grep`);
- every `let ClassMember::Method(m) = member;` (an *irrefutable* let today,
  including several written in the 174 pass arms) **refutable** → each must become
  a `match`/`if let`.

The correct `Field` arm **recurses into the field `value`** (a normal expression —
fold it, resolve references in it, rewrite renamed uses in it) and into a
computed key's expression, mirroring how the `Method` arm recurses into the method
body. A field **key** is a property name: the property-renaming pass renames it
exactly like a method key (no `constructor` special-case — a field is never the
constructor), and the variable-renaming passes leave it untouched. Passes that
only walk *executable* positions (dce's dead-code predicates, inline's call
tally) treat a field like a method — its initializer is code that runs at
construction, so a candidate use inside it must still be counted.

### PR breakdown

- **PR1 (atomic node + emit + all `ClassMember` arms, ONE commit).** `javascript-ast`
  (MINOR) adds the `Field` variant + `PropertyDefinition`; `closure-emitter` (MINOR)
  adds the `Field` arm to `emit_class_member`/`emit_class_tail` printing
  `[static ]key[=value];`; every pass crate whose `ClassMember` match went
  non-exhaustive gets its `Field` arm (mirroring the `Method` arm's recursion into
  the value + computed key), and every now-refutable `let ClassMember::Method`
  becomes a `match`. Hand-constructed emitter unit tests (bare field, `=value`,
  `static`, computed key, field+method interleaved).
- **PR2 (bridge-enable).** `javascript-parser` (MINOR) + `closurec` (PATCH):
  convert the grammar's field `class_element` production → `ClassMember::Field`.
  Dump the parse-tree shape first. Add a `closurec` e2e fixture
  (`tests/diff/simple-class-field/`). Decline grammar-unsupported field shapes to
  WHITESPACE_ONLY (never a miscompile).
- **PR3 (CodePrinter conformance port).** `closure-emitter` (PATCH): new
  `tests/upstream/code_printer_class_field_test.rs` mirroring upstream
  `CodePrinterTest`'s class-field printing cases — hand-constructed AST, so it also
  covers computed / static / no-initializer shapes the grammar may not yet parse.

Static-init blocks (`static { … }`), private names (`#x`), and decorators remain
deferred to their own additive slices, exactly as for the method surface.

### PR1 (DONE)

Landed as the atomic node + emit + all-arms commit. `ClassMember` is now a
two-variant enum (`Method` | `Field`). Confirmed blast radius (per the per-crate
build, the ground truth):

- **`javascript-ast` 0.34.0** — `ClassMember::Field(PropertyDefinition)` +
  `PropertyDefinition { cv, key: PropertyKey, value: Option<Expression>, computed,
  is_static }`; 4 roundtrip tests (initialized / bare / static / computed-key,
  the last using a `[a+b]` `BinaryExpression` key so it is unambiguously
  `PropertyKey::Expression` rather than collapsing to `Identifier` under the
  untagged serde — the same limitation methods already have).
- **`closure-emitter` 0.39.0** — `emit_class_field` (printing `[static ]key[=value];`,
  value at `PREC_ASSIGNMENT`) wired into the shared `emit_class_tail` member loop;
  6 emit tests.
- **Pass crates (PATCH each), `Field` arms added at every site the compiler
  flagged non-exhaustive/refutable:** constant-fold (1), fold-control-flow (1),
  dce (2), scope-analyzer (2), rename-properties (2), inline-variables (5),
  rename-globals (5), rename (4: 2 decl + 2 expression handlers), **inline (13:**
  the widest — the soundness-critical `tally`/`inline`/`mutated-params`/
  `used-idents` sites recurse the initializer + computed key, the scope-aware
  `substitute`/`rename` sites use the class-inner map, and `count_decl_names` +
  both `splice_*_in_decl` correctly skip a field**).** All 11 crates build + test
  green.

The `Field` arm everywhere mirrors the `Method` arm's recursion but on
`f.value: Option<Expression>` (+ the computed-key expression) rather than a
statement body, exactly as this section specified. Bridge + conformance follow in
PR2/PR3.

## CLOC12.174 — `ClassDeclaration` (`class C [extends S] { … }`): the class *statement* arc (design)

CLOC12.173 shipped the class **expression** (a value: `x = class {}`, `f(class C {})`).
Its closing note flagged the remaining half explicitly: *"The bridge for a class
**declaration** (a `ProgramItem`, not an `Expression`) is a separate future arc…
The two share the member sub-AST once PR1 lands."* That arc is CLOC12.174, and the
member sub-AST (`ClassMember` / `MethodDefinition` / `MethodKind`) now exists, so
this is a **small, additive** arc — no new member modelling, only a new
*declaration-position* node that reuses it.

A class declaration is the **statement** form: `class C { … }` written where a
statement is expected, binding the name `C` in the enclosing scope. It mirrors
`FunctionDeclaration` (the `function f(){}` statement) exactly as `ClassExpression`
mirrors `FunctionExpression`.

### Model (ESTree-aligned)

```rust
// javascript-ast/src/declaration.rs — a new variant of the `Declaration` enum
pub enum Declaration {
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),          // NEW
}

pub struct ClassDeclaration {
    pub cv: Option<CvId>,
    pub id: Identifier,                          // REQUIRED — a declaration always binds a name
    pub super_class: Option<Box<Expression>>,    // `extends <expr>` heritage (reused shape)
    pub body: Vec<ClassMember>,                  // reuse ClassMember from expression.rs
}
```

**The one structural difference from `ClassExpression`: `id` is `Identifier`, not
`Option<Identifier>`.** A class *expression* may be anonymous (`x = class {}`); a
class *declaration* must name its binding (`class {}` in statement position is a
syntax error), exactly as `FunctionDeclaration.id` is required where
`FunctionExpression.id` is optional. `super_class` and `body` are byte-identical
in shape to `ClassExpression` and reuse the same `ClassMember` / `MethodDefinition`
/ `MethodKind` types (imported from `expression.rs`) — no new member modelling.

### Emit (`closure-emitter`)

`class <id>[ extends S]{members}` — the same body shape as `emit_class`, with three
deliberate differences from a class *expression*, all mirroring the
declaration/expression split already established for functions:

1. **No precedence wrap and no statement-start parenthesisation.** A class
   expression is tagged `PREC_UNARY` and `emit_expression_statement` wraps a
   leading one (`(class{});`) because a statement-position `class` would otherwise
   parse as a *declaration* — which is precisely what a `ClassDeclaration` **is**.
   So the declaration form is emitted bare, with no `expr_prec` entry and no
   wrap-set membership.
2. **`id` always prints** (it is non-optional), with a `required_ws()` after
   `class` exactly as `emit_function_declaration` does after `function`.
3. **No trailing `;`.** `emit_function_declaration` appends a normalising `;`
   after its `}` (gap-030 part B). A **class** declaration does *not* — upstream
   Closure terminates a `ClassDeclaration` with its `}` alone (a class body is
   self-delimiting and never subject to ASI hazards the way a bare
   `function(){}` value is). PR3's conformance port validates this against
   `CodePrinterTest`.

Implementation reuses `emit_class_member` unchanged. The shared
`[ extends S]{members}` tail is factored into a small helper used by **both**
`emit_class` (after its optional-id head) and the new `emit_class_declaration`
(after its required-id head), so the heritage-precedence + member-loop logic lives
in one place.

### Pass arms — the atomic-PR1 blast radius

Adding a variant to the `Declaration` enum makes **every exhaustive `match` on
`Declaration` fail to compile** until an arm is added — that is the point of the
atomic PR1 (enum + emit + all arms in ONE commit, workspace green). The
`Declaration` match sites are: `closure-emitter`, `closure-pass-constant-fold`,
`closure-pass-dce`, `closure-pass-fold-control-flow`, `closure-pass-inline`,
`closure-pass-inline-variables`, `closure-pass-rename`, `closure-pass-rename-globals`,
`closure-pass-rename-properties`, `closure-pass-treeshake`,
`closure-pass-remove-unused-vars`, `closure-scope-analyzer`, plus `bridge.rs` /
`run.rs` (touched in PR2, not PR1). Each new arm **mirrors that crate's existing
`Expression::ClassExpression` handling** — which already knows how to walk
`super_class` and each method `value` (a nested function scope) — since 173 PR1
added exactly that machinery. So the arm is "recurse into the heritage operand and
each member body," reusing the same helper the ClassExpression arm calls. Rebuild
/ transform arms delegate to an `#[inline(never)]` helper (frame-size DoS lesson,
per 173). The declared *name* `C` is a binding a variable-renaming pass may rename
(like `FunctionDeclaration.id`), unlike a method key (a property name) — the arm
follows the `FunctionDeclaration` arm's treatment of `.id`, not the method-key
treatment. Non-exhaustive matches with a catch-all (`treeshake` /
`remove-unused-vars` may already absorb it) need no arm; the per-crate CI
build-tool is the source of truth for which do (a compile error there — NOT a
green `grep -c "test result: FAILED"`, the flawed check called out in 173 — is
what proves the arm was needed).

### PR breakdown

- **PR1 (atomic node + emit + all pass arms, ONE commit).** `javascript-ast`
  (MINOR) adds `ClassDeclaration` + the `Declaration::ClassDeclaration` variant;
  `closure-emitter` (MINOR) adds `emit_class_declaration` + the shared class-tail
  helper + the `Declaration::ClassDeclaration` emit arm; every pass crate that
  matches `Declaration` exhaustively gets its mirror-of-FunctionDeclaration arm.
  Hand-constructed emitter unit tests (named class, `extends`, one method,
  `static`/`get`/`set`, computed key, `constructor` — asserting **no** trailing
  `;` and **no** wrapping paren).
- **PR2 (bridge-enable).** `javascript-parser` (MINOR) + `closurec` (PATCH):
  convert the grammar's `class_declaration` node → `Declaration::ClassDeclaration`,
  reusing the `convert_class_heritage` / `convert_class_element` /
  `convert_method_definition` converters that 173 PR2 already built for the
  expression form (only the top-level node kind and the required-`id` differ).
  Dump the parse-tree shape first. Add a `closurec` e2e diff fixture
  (`tests/diff/simple-class-decl/`). Same decline-to-WHITESPACE_ONLY safety for
  the member shapes the grammar can't yet cleanly bridge (generator / async /
  computed / multi-member), never a miscompile.
- **PR3 (CodePrinter conformance port).** `closure-emitter` (PATCH): new
  `tests/upstream/code_printer_class_declaration_test.rs` mirroring upstream
  `CodePrinterTest`'s class-**declaration** printing cases (statement position, no
  wrap, no trailing `;`, heritage, members) — hand-constructed AST, so it also
  covers the member shapes the grammar cannot yet parse.

Serialised after 173 (shares the member sub-AST it introduced). Fields
(`PropertyDefinition`) and static blocks remain deferred to their own additive
slices, identical to the ClassExpression deferral.

**PR1 (DONE).** `javascript-ast` 0.33.0 adds `ClassDeclaration` + the
`Declaration::ClassDeclaration` variant exactly as modelled; `closure-emitter`
0.38.0 adds `emit_class_declaration` + the shared `emit_class_tail` helper
(factored out of `emit_class`), with 5 unit tests asserting the bare, no-`;`,
no-wrap shape. The atomic pass-arm set turned out **larger than the design note's
match-site list**: the note named 12 crates but only enumerated the ones with a
central `fn *_declaration`; in fact `closure-pass-inline` (7 sites) and
`closure-pass-rename` (5 sites) ALSO match `Declaration` exhaustively at many
predicate/collect/rewrite helpers, and both needed arms — the per-crate CI
build-tool (a compile error, the ground truth) is what surfaced them, exactly the
lesson 173 recorded. Final arm-bearing crates: `constant-fold` 0.85.14, `dce`
0.20.14, `fold-control-flow` 0.20.14, `inline` 0.25.14, `inline-variables`
0.11.14, `rename` 0.14.14, `rename-globals` 0.10.14, `rename-properties` 0.12.14,
`scope-analyzer` 0.12.14. `treeshake` / `remove-unused-vars` route through
catch-alls (no arm). Each arm mirrors the crate's existing
`Expression::ClassExpression` handling plus the required class-name binding;
soundness-critical passes (`inline` tally/inline, `inline-variables`
count-uses/propagate, `rename` collect/rewrite) recurse the heritage + every
method body so a class use is never missed. Where a class-body walk was
duplicated between the expression and declaration forms it was factored into a
shared helper (`fold_class_body`, `classify_class_members` /
`rewrite_class_members`). Reachable end-to-end once the PR2 bridge produces the
node.

**PR2 (DONE) — `javascript-parser` 0.38.0 + `closurec` 0.234.13.** The bridge's
`convert_source_element` converts a top-level `class_declaration` →
`Declaration::ClassDeclaration` via new `convert_class_declaration`, so a class
declaration flows through the full pipeline instead of declining to
WHITESPACE_ONLY. **The design held with one grammar-shape refinement found by
dumping the parse tree:** at the `source_element` level the node is wrapped in
`decorated_class_declaration` → `class_declaration` (the outer rule that would
also carry `@decorator`s), so the source-element arm unwraps it (and declines a
genuinely *decorated* form — a later slice). The inner `class_declaration` node's
flat child shape is **identical to `class_expression`** (`class` / NAME / optional
`class_heritage` / `class_body`) save the required name, so
`convert_class_declaration` reuses `convert_class_heritage` /
`convert_class_element` unchanged and DECLINES a nameless class rather than
fabricate an empty id. Generator / async / computed / multi-member methods decline
to WHITESPACE_ONLY exactly as for the expression form. 10 bridge unit tests
(`class_decl_*`); `closurec` e2e fixture `tests/diff/simple-class-decl/`
(`class C { m() { return 1 + 2 } }` → `class C{m(){return 3}}`) proves the class
round-trips, the method body folds, and the declaration emits **bare** (no
trailing `;`, no wrapping paren).

**PR3 (DONE) — `closure-emitter` 0.38.1.** New upstream port
`tests/upstream/code_printer_class_declaration_test.rs` (registered as `[[test]]
upstream_code_printer_class_declaration`), the companion to the class-expression
port. Isolates `emit_class_declaration` + the shared `emit_class_tail` helper from
PR1. **20 active `#[test]`s, 0 `#[ignore]`** — the declaration emits bare (no wrap,
no trailing `;`), the four `extends`-operand precedence cases (identifier / member
/ call bare, conditional wrapped `extends (a?b:c)`), the member forms (method /
params+body / `static` / `get` / `set` / `constructor` / stacked `static get` /
generator `*m` / `async m` / computed `[k]`, `[0]`, `[a+b]` / two members
back-to-back), and the full shape `class C extends B{m(){}}`. Inputs are
hand-constructed AST, so the port also covers the generator / async / computed-key
/ multi-member shapes the grammar cannot yet parse. Test-only change (no
production edit); ATTRIBUTION.md updated. **This closes the CLOC12.174
class-declaration arc.** Deferred to their own additive slices (as for the
expression form): instance/static class **fields** (`PropertyDefinition`), static
initialization blocks, private names, decorators.

## CLOC12.173 — `ClassExpression` (`class [id] [extends S] { … }`): the class arc (design)

`ClassExpression` is the next `Expression` node, and unlike every arc since
CLOC12.150 it is **greenfield**: the typed AST models classes *nowhere* today
(`declaration.rs` still says "Phase 3 adds `ClassDeclaration`"; no class struct
exists). The grammar already parses `class_expression` / `class_declaration` /
`class_body` / `class_element` / `method_definition`, but the bridge declines
all of them (`bridge.rs` `convert_primary` maps `class_expression` →
`unsupported`; `method_definition` is "unsupported Phase 2"). So a class literal
currently drops its whole file to `WHITESPACE_ONLY`.

Because a class carries a **member sub-AST**, this is larger than a single leaf
node. This section fixes the model and the PR breakdown so the implementation
lands in reviewable, atomic pieces rather than one monolith.

### Model (ESTree-aligned, Rhino-informed)

```rust
// javascript-ast/src/expression.rs
pub struct ClassExpression {
    pub cv: Option<CvId>,
    pub id: Option<Identifier>,               // `class C {}` vs anonymous `class {}`
    pub super_class: Option<Box<Expression>>, // `extends <expr>` heritage
    pub body: Vec<ClassMember>,               // the `{ … }` element list (may be empty)
}

pub enum ClassMember {
    Method(MethodDefinition),
    // Field(PropertyDefinition) and StaticBlock(...) arrive in later feature PRs.
}

pub struct MethodDefinition {
    pub cv: Option<CvId>,
    pub key: PropertyKey,          // reuse the object-literal key type
    pub kind: MethodKind,          // constructor | method | get | set
    pub value: FunctionExpression, // params + body live here (as for object methods)
    pub computed: bool,            // `[expr]() {}` (mirrors `Property.computed`)
    #[serde(rename = "static")]
    pub is_static: bool,           // `static m() {}`
}

pub enum MethodKind { Constructor, Method, Get, Set }
```

**Why separate `MethodDefinition`, not reuse object-literal `Property`.** A
`Property` has `shorthand`/`method`/`Init|Get|Set` and a `value: Expression`; a
class method additionally needs `static`, a `constructor` kind (which a
`Property` cannot express), and its `value` is *always* a `FunctionExpression`.
This matches the conformance target — Closure's Rhino uses dedicated
`MEMBER_FUNCTION_DEF` / `GETTER_DEF` / `SETTER_DEF` / `COMPUTED_PROP` node kinds,
not object-property nodes. `PropertyKey` (identifier / string / numeric /
`Expression` for computed) *is* reused — a method key and a property key have
the same four shapes.

**Deliberately deferred to later feature PRs (each an additive `ClassMember`
variant or `MethodDefinition` field):** instance/static **fields**
(`PropertyDefinition`), **static initialization blocks** (`static { … }`),
private names (`#x`), and decorators. Excluding them keeps PR1 cohesive and
each later slice small — the same "ship minimal, grow additively" path
TemplateLiteral (no-substitution first) and optional chaining followed.

### PR breakdown

- **PR1 (atomic node + emit + all pass arms, ONE commit).** Add the
  `ClassExpression` variant + the member sub-AST above to `javascript-ast`
  (MINOR); add `emit_class` to `closure-emitter` (MINOR) printing
  `class[ id][ extends S]{members}` with each member via an `emit_class_member`
  helper (`static`/`get`/`set`/`*`-generator prefixes, computed `[key]`,
  method params+body via the existing function-emit path), classified
  `PREC_PRIMARY` (a class expression in a tighter parent — e.g. a call callee
  `(class{})()` — wraps, exactly like a function expression); and add a
  `ClassExpression` arm to **every** exhaustive `Expression` match in the pass
  crates. The correct arm **mirrors `FunctionExpression`**: a class is a
  scope-introducing, code-bearing node, so scope-analyzer / rename /
  rename-globals / rename-properties / inline / inline-variables / dce /
  fold-control-flow / constant-fold must recurse into `super_class` and each
  method's `value` (a nested function scope) rather than treat the class as an
  inert leaf. Check `treeshake` / `remove-unused-vars` / `collapse-properties`
  for catch-all arms that already absorb it. Hand-constructed AST unit tests in
  the emitter (empty class, named, `extends`, one method, `static`/`get`/`set`,
  computed key).
- **PR2 (bridge-enable).** `javascript-parser` (MINOR) + `closurec` (PATCH):
  convert `class_expression` → `ClassExpression`, walking `class_body` /
  `class_element` / `method_definition` into `ClassMember::Method`. Dump the
  parse-tree shape first (the env-gated `eprintln` + throwaway probe technique).
  Add a `closurec` e2e diff fixture (`tests/diff/simple-class/`). Closes the
  `class_expression`-declines-to-WHITESPACE_ONLY gap.
- **PR3 (CodePrinter conformance port).** `closure-emitter` (PATCH): new
  `tests/upstream/code_printer_class_test.rs` mirroring upstream
  `CodePrinterTest`'s class printing cases.

The bridge for a **class _declaration_** (a `ProgramItem`, not an
`Expression`) is a **separate future arc** — this arc is scoped to the
*expression* form. The two share the member sub-AST once PR1 lands.

**PR1 (DONE).** `javascript-ast` 0.32.0 adds the node + member sub-AST exactly
as modelled above; `closure-emitter` 0.37.0 adds `emit_class` /
`emit_class_member` (with a shared `emit_param_list_and_body` helper factored
out of `emit_function_expression`); `closure-pass-constant-fold` 0.85.13 folds
inside the `extends` operand and method bodies via an `#[inline(never)]`
`fold_class` helper. 8 emitter unit tests. **Two refinements to the plan above,
found during implementation:**

1. **Precedence is `PREC_UNARY`, not `PREC_PRIMARY`.** The plan said
   `PREC_PRIMARY`, but `FunctionExpression` — the node a class expression
   mirrors — is tagged `PREC_UNARY` in `expr_prec`, and *that* is what makes it
   wrap as a member object (`(class{}).x`) or call callee (`(class{})()`);
   `PREC_PRIMARY` would leave it bare and mis-emit. `ClassExpression` therefore
   also tags `PREC_UNARY`, and is added to `emit_expression_statement`'s
   leading-token wrap set (a statement-position `class` parses as a class
   *declaration*, like `function`/`{`).
2. **Most pass crates DO match `Expression` exhaustively and each needed a
   `ClassExpression` arm.** (An earlier draft of this note wrongly said "only
   two crates" — that conclusion came from a flawed local check: `grep -c
   "test result: FAILED"` returns `0` for a crate that failed to *compile*, so a
   non-exhaustive-match compile error read as a pass. The per-crate CI build-tool
   caught it; see lessons.md.) In fact `closure-emitter`, `closure-pass-constant-fold`,
   `closure-scope-analyzer`, `closure-pass-dce`, `closure-pass-fold-control-flow`,
   `closure-pass-inline-variables`, `closure-pass-inline`, `closure-pass-rename`,
   `closure-pass-rename-globals`, and `closure-pass-rename-properties` all match
   `Expression` exhaustively (several at 2+ sites, some over `&mut Expression`),
   and each got a `ClassExpression` arm **mirroring its `FunctionExpression`
   handling** — recurse into the `extends` operand (a normal expression) and each
   method’s `value` (a `FunctionExpression`, walked as its own function scope).
   Variable-renaming passes leave method *keys* untouched (a method key is a
   property name, not a variable); the property-renaming pass treats method keys
   as renameable property names, mirroring object-literal keys. Rebuild/transform
   arms delegate to an `#[inline(never)]` helper (frame-size DoS lesson). Only
   `closure-pass-treeshake` / `-remove-unused-vars` / `-collapse-properties` route
   through catch-alls (no arm needed). This is reachable only once the PR2 bridge
   produces the node; the arms give correct (not just compiling) behaviour when it
   does, so no optimisation-inside-class work is deferred beyond the deferred
   *node features* (fields / static blocks).

**PR2 (DONE) — `javascript-parser` 0.37.0 + `closurec` 0.234.12.** The bridge's
`convert_expression` now converts `class_expression` → `Expression::ClassExpression`
via new `convert_class_expression` / `convert_class_heritage` /
`convert_class_element` / `convert_method_definition` converters, so a class
expression flows through the full pipeline instead of declining to
WHITESPACE_ONLY. `tests/diff/simple-class/` (SIMPLE) proves the class round-trips
and the method body + sibling arg fold; `tests/diff/advanced-class-constructor/`
(ADVANCED) is the end-to-end regression for the PR1 `constructor` no-rename guard.
**Three divergences from the plan above, found by dumping the parse tree:**

1. **Heritage operand comes in two shapes.** `class_heritage` holds a bare
   `NAME` token for `extends B` (→ `Identifier`) but a
   `left_hand_side_expression` *node* for `extends ns.B` (→ `convert_expression`).
   `extends <call>` (`mix(B)`) flattens to several ambiguous NAME tokens with no
   clean operand node, so it DECLINES rather than mis-read the super-class.
2. **`async` is a distinct node, not a leading token.** A generator `*m(){}`
   puts `*` inside `method_definition`, but an `async` method parses as a
   separate `async_method` node under `class_element`. So `convert_class_element`
   declines any member node that is not a plain `method_definition` (a
   node-kind check, not just a token scan) — otherwise the `async` would be
   silently dropped, a miscompile. Generator and computed-`[k]()` methods
   likewise decline (later slices). All declines fall back to WHITESPACE_ONLY —
   never a miscompile.
3. **Grammar requires `;` between members.** `class { m(){} n(){} }` (no
   separator) is a *parse* error → WHITESPACE_ONLY fallback; single-member
   classes parse and bridge cleanly. This is a grammar limitation, not a bridge
   decision, and is the reason both e2e fixtures use single-member classes.

16 bridge unit tests (`class_*`) cover the accepted forms and the declines.

**PR3 (DONE) — `closure-emitter` 0.37.1.** New upstream port
`tests/upstream/code_printer_class_test.rs` (registered as `[[test]]
upstream_code_printer_class`), isolating `emit_class` + `emit_class_member` +
the `PREC_UNARY` classification from PR1. **22 active `#[test]`s, 0
`#[ignore]`** — the statement-start wrap (`(class{});`), anonymous/named
surface, the four `extends`-operand precedence cases (identifier / member / call
heritage bare, conditional heritage wrapped `extends (a?b:c)`), the member forms
(empty / params+body / `static` / `get` / `set` / `constructor` / stacked
`static get` / generator `*m` / `async m` / computed `[k]` / two members
back-to-back), and the whole-node precedence cases (`(class{}).x`,
`(class{})()`, `class{}+1`). Inputs are hand-constructed AST, so the port also
covers the generator / async / computed-key / multi-member shapes the grammar
cannot yet parse. Test-only change (no production edit); ATTRIBUTION.md updated.
This closes the CLOC12.173 class-expression arc.

## CLOC12.172 — `RegExpLiteral` leaf node (`/pattern/flags`): node + emit + passes (PR1)

Regex literals were previously bridged via a **"treat as identifier" fallback**
(`javascript-parser` `convert_primary`'s catch-all) — an `Identifier` whose name
was the regex source. It round-tripped only by accident (the text is not a valid
identifier, so the rename passes skipped it as a candidate) and left a latent
hazard: any pass that treated it as a real reference could corrupt it.

PR1 models it properly as `Expression::RegExpLiteral(RegExpLiteral { cv, pattern,
flags })` (`javascript-ast` 0.31.0) — a leaf beside the other literal leaves.
The two halves are stored split (`pattern` between the slashes, `flags` the
trailing set) so passes reason about the flags without re-parsing.
`closure-emitter` (0.36.0) `emit_regexp` reconstructs `/{pattern}/{flags}` with
no escaping (a regex has one spelling) at `PREC_PRIMARY`; 3 unit tests. All
downstream pass/analysis crates group `RegExpLiteral` into their existing inert
leaf-literal arm (same treatment as `StringLiteral`/`NumericLiteral`: no
recursion, no renaming, not a foldable constant) — several already covered it
via a `_` catch-all. All in ONE atomic commit. The bridge that *builds* the node
(replacing the identifier fallback, closes gap-RegExpAsIdentifier) is PR2; the
CodePrinter conformance port is PR3.

**PR2 (DONE, closes gap-RegExpAsIdentifier).** `javascript-parser` 0.36.0's
`convert_primary_token` now recognises the lexer's REGEX token — discriminated
exactly like BIGINT (`type_ = Name`, `type_name = Some("REGEX")`, `value =
"/pat/flags"`, verified by a throwaway parse-tree dump) — and builds a real
`Expression::RegExpLiteral` instead of the identifier catch-all. A new
`split_regex_literal` helper splits the token value around the **closing**
delimiter, scanning from index 1 while honouring `\`-escapes (`\/` is literal)
and character classes (`[...]`, inside which `/` is literal), so pattern/flags
are extracted correctly for `/a\/b/`, `/[abc]/`, `/ab+c/gi`. Debug-build asserts
guard no-raw-newline-in-pattern and flags ⊆ `[dgimsuy]`. closurec 0.234.11's
`tests/diff/simple-regex/` (`f(/ab+c/gi, 1 + 2);` → `f(/ab+c/gi,3);`) proves the
literal round-trips through the full SIMPLE pipeline and the sibling `1 + 2`
folds. **Discovered lexer gap (separate item):** the splitter handles `/[/]/`
(a `/` inside a class) correctly, but the lexer does not yet tokenise that shape
— it stops the literal at the inner `/`. End-to-end `/[/]/` support is a lexer
change, not a bridge concern.

**PR3 (DONE).** `closure-emitter` 0.36.1 adds
`tests/upstream/code_printer_regexp_test.rs` (the nineteenth CodePrinter port),
isolating `emit_regexp` + the `PREC_PRIMARY` classification. 13 active `#[test]`s
/ 0 `#[ignore]`, mirroring upstream's verbatim `Token.REGEXP` printing: a regex
has exactly one spelling, so the delimiters and raw pattern/flags are written
verbatim with no quote-choice or re-escaping. Covers pattern+flags round-trip,
no-flags bare form, opaque pattern bodies (groups/alternation, character class,
anchors+quantifiers, backreference, `/` inside a class, escaped delimiter),
verbatim flags (full `dgimsuy` set + non-canonical order echoed unchanged), and
the `PREC_PRIMARY` composition cases (paren-free member base `/re/.test(a)` /
`/re/g.source`, bare call argument `f(/ab+c/gi,1)`). Emitter-tests-only. This
completes the CLOC12.172 RegExpLiteral arc.

## CLOC12.171 — optional chaining `a?.b` / `a?.[k]` / `a?.()`: nodes + emit + passes (PR1)

Optional chaining (ES2020) lets a member/call short-circuit to `undefined`
instead of throwing when the base is `null`/`undefined`: `a?.b`, `a?.[k]`,
`a?.()`. Three new `Expression` variants model it (`javascript-ast` 0.30.0):

```rust
OptionalMemberExpression { cv, object, property, computed }  // a?.b / a?.[k]
OptionalCallExpression    { cv, callee, arguments }          // a?.()
ChainExpression           { cv, expression }                 // chain boundary
```

**Modelling — separate node types, not an `optional: bool` flag (a deliberate
divergence from the ESTree-7 shape).** ESTree-7 marks optional links by setting
`optional: true` on the ordinary `MemberExpression`/`CallExpression` and wraps
the chain in `ChainExpression`. We instead give the optional links their **own
node types** for two reasons: (1) it matches the conformance target — Google's
Closure Compiler uses dedicated Rhino node kinds `OPTCHAIN_GETPROP` /
`OPTCHAIN_GETELEM` / `OPTCHAIN_CALL`, not a flag on the ordinary access nodes;
and (2) it is purely additive — `MemberExpression`/`CallExpression` have ~150
construction sites across the workspace, so a new required field would touch
every one, whereas a new variant only adds `match` arms where a pass cares.
`ChainExpression` is kept as the short-circuit-boundary marker (it prints
transparently). A non-optional suffix that follows an optional one stays an
*ordinary* `MemberExpression`/`CallExpression` whose object/callee is the
optional node, so `a?.b.c` needs no per-link flag — only the `b` link carries
`?.`.

`closure-emitter` (0.35.0) prints them: `emit_optional_member` spells `?.` /
`?.[`…`]`, `emit_optional_call` spells `?.(`…`)`, and `emit_chain` descends
transparently into its inner expression. All three are `PREC_PRIMARY` (they
compose like member/call), so the object/callee binds at primary strength and a
looser object keeps its parens (`(a||b)?.c`); a looser *sequence* call argument
wraps (`a?.((b,c))`). 5 emitter unit tests.

All downstream pass/analysis crates gain their three match arms — the optional
member/call arms mirror the plain member/call handling (recurse into
object/property, callee/arguments), and the `ChainExpression` arm recurses into
its single inner expression. All in ONE atomic commit so the workspace never
has a broken `match`. The grammar→typed-AST bridge that *builds* these nodes
from `optional_chain_expression` is CLOC12.171 PR2 (closes gap-OptionalChain);
the CodePrinter conformance port is PR3.

### gap-OptionalChain — bridge declines optional chaining `a?.b` / `a?.[k]` / `a?.()` — **RESOLVED (javascript-parser 0.35.0, CLOC12.171 PR2)**

Before PR2, `convert_optional_chain_expression` (and the sibling call/member
chain converters) bailed with `UnsupportedSyntax` the moment any `?.` token
appeared, so every optional chain dragged the whole file to WHITESPACE_ONLY.

**Fix (CLOC12.171 PR2):** an env-gated parse-tree dump confirmed the grammar
spells `?.` as its own token followed directly by the suffix — `?.` NAME (dot),
`?.` `[` expr `]` (computed), `?.` `arguments` (call). The suffix walker gained
a `?.` arm that builds `OptionalMemberExpression` / `OptionalCallExpression` for
the marked link (a following non-optional suffix like the `.c` in `a?.b.c` stays
an ordinary member/call whose object is the optional node), and the whole spine
is wrapped once in a `ChainExpression` when any optional link appeared. A
non-optional chain is returned bare. 5 bridge unit tests + the closurec
`simple-optchain` e2e fixture (`f(a?.b, 1 + 2);` → `f(a?.b,3);`).

**Remaining (safe) follow-up:** `f()?.x` — an optional link whose *base* is a
call — routes through `convert_call_expression`, whose `?.` decline arm still
falls back to WHITESPACE_ONLY (never a miscompile).

## CLOC12.170 — object spread `{...o}` via `ObjectMember`: structural node + emit + passes (PR1)

Object literals now model **object spread** `{...o}` (ES2018) — the `...expr`
member that copies another object's own enumerable properties in. This is the
FIRST *structural* node in the CLOC12 arc: rather than adding an `Expression`
variant, it changes the shape of an existing node. `ObjectExpression.properties`
goes from `Vec<Property>` to `Vec<ObjectMember>` (`javascript-ast` 0.29.0):

```rust
pub enum ObjectMember { Property(Property), Spread(SpreadElement) }
```

The spread arm **reuses** the existing `SpreadElement { cv, argument }` — the
same node the call/array spread (CLOC12.162) uses, which ESTree also names
`SpreadElement` — so there is one spread shape and one emit path. `Property` and
`Spread` members share ONE ordered vector because the order is observable: in
`{a: 1, ...o, b: 2}` a later member overrides an earlier key, and a spread may
sit before, between, or after plain properties. A side channel would lose the
interleaving and silently miscompile override semantics.

`closure-emitter` (0.34.0) `emit_object` iterates the members and prints a
spread via `emit_object_spread` — `...` then the argument at `PREC_ASSIGNMENT`
with no interior space, mirroring the call/array `emit_spread`. The assignment
precedence is the crux: an object-member position is an `AssignmentExpression`,
so everything at or above assignment prints bare (`{...a}`, `{...a.b}`,
`{...f()}`) while the one looser form — a **sequence** — wraps (`{...(a,b)}`),
because a bare `...a,b` would spread only `a` and leave `,b` as a second
(invalid) member slot. 5 emitter unit tests.

All nine downstream pass/analysis crates gain their `ObjectMember` match arms —
transform passes (`constant-fold`, `dce`, `fold-control-flow`) rebuild both
member kinds and recurse into the spread `argument`; visitor passes (`inline`,
`inline-variables`, `rename`, `rename-globals`, `rename-properties`,
`scope-analyzer`) walk the spread `argument` like any read sub-expression
(a spread has no property name, so the name-renaming passes never rename it).
`Object.keys`/`Object.entries` constant-folds decline (return `None`) when any
member is a spread, since a spread injects statically-unknown keys. All in ONE
atomic commit so the workspace never has a broken `match`.

### gap-SpreadProperty — bridge declines object spread `{...o}` — **RESOLVED (javascript-parser 0.34.0, CLOC12.170 PR2)**

The `javascript-parser` bridge's `convert_property_definition` declined a `...`
spread property with `UnsupportedSyntax { rule: "SpreadProperty" }`, so a file
containing `{...o}` dropped to WHITESPACE_ONLY. PR1 landed the `ObjectMember`
node + emit + pass recursion so the typed AST and every pass could already
represent and print an object spread.

**Fix (CLOC12.170 PR2):** dumping the parse tree showed the object-spread form
nests one level deeper than the call/array spread (gap-163): a
`property_definition` holds a single `object_spread_property` Node child whose
own children are `[ Token("..."), Node(assignment_expression) ]` (the call/array
spread's ELLIPSIS sits directly under `spread_element`, a different rule). So
`convert_object_literal` now detects the spread by that inner rule name — it
finds an `object_spread_property` child, extracts the `assignment_expression`
(via `node_children`, which strips the ELLIPSIS token), converts it, and wraps
it in `ObjectMember::Spread(SpreadElement { .. })` — reusing the same
`SpreadElement` the call/array spread uses so it prints through
`emit_object_spread`. Member order is preserved (observable: a later member
overrides an earlier key). The dead `SpreadProperty` decline arm is removed from
`convert_property_definition`. 3 bridge unit tests, plus the closurec e2e diff
fixture `tests/diff/simple-objspread/` (`f({...o, x: 1 + 2});` → `f({...o,x:3});`)
proving `{...o}` flows through SIMPLE. The **PR3** CodePrinter conformance port
follows.

## CLOC12.169 — `ImportExpression` (dynamic `import(x)`): atomic node + emit + passes (PR1)

Adds `Expression::ImportExpression { cv, source }` to `javascript-ast` (0.28.0)
— a **dynamic `import(specifier)`**, the runtime module-loading form that
returns a promise for the module namespace. It is distinct from a static
`import` *declaration* (a statement) and from the `import.meta` meta-property
(the `ImportMeta` leaf, CLOC12.168). ESTree names this node `ImportExpression`
with a single `source` operand.

This is the first **compound** Expression node in the recent run of reserved-word
leaf primaries (`this` / `super` / `new.target` / `import.meta`): it carries one
sub-expression, the module specifier, so `source` is a non-optional
`Box<Expression>` — the same single-operand shape as `AwaitExpression`
(`import()` with no argument is a syntax error). There is no second axis
(the import-attributes / options-bag proposal is not modelled).

**Emit (`closure-emitter` 0.33.0).** `emit_import_expression` prints the
`import` keyword immediately followed by a *literal* parenthesised argument — a
call-like primary. Unlike the word-shaped unary `await`, no separator follows
the keyword (`import(x)`, never `import (x)`). The `source` sits inside the
literal parens, emitted at `PREC_ASSIGNMENT` (the call-argument level): a looser
*sequence* specifier wraps (`import((a,b))`), everything else prints bare
(`import("m")`, `import(a.b)`, `import(f())`). The whole node tags at
`PREC_PRIMARY` — atomic from the outside, like a `CallExpression` — so a
member/call parent composes without extra parens (`import(x).then(f)`).

**Passes (PATCH bumps).** Unlike the leaf primaries (whose pass arms are no-ops),
a dynamic import has a child, so every pass **recurses into `source`** exactly
like the sibling `AwaitExpression` arm: the three rebuild passes
(`constant-fold`, `dce`, `fold-control-flow`) rebuild `ImportExpression` with a
folded `source` (and `fold-control-flow` returns its `cv` from the
`expression_cv` accessor); the six traversal passes (`inline`,
`inline-variables`, `rename`, `rename-globals`, `rename-properties`,
`scope-analyzer`) visit / rewrite the `source` sub-expression. Atomic node PR1:
node + emit + all nine downstream pass arms land together so the workspace never
breaks.

### gap-170 — bridge declines dynamic `import(x)` (`ImportExpression`) — **RESOLVED (javascript-parser 0.33.0, CLOC12.169 PR2)**

The atomic node PR (CLOC12.169 PR1) landed the node + emit + pass recursion so the
typed AST and every downstream pass can already represent, optimise-through, and
print a dynamic import. The **PR2 probe** confirmed the grammar *already* produces
a `dynamic_import` node with children `[Token("import"), Token("("),
Node(source_expr), Token(")")]` — no grammar work was required (unlike `await`,
gap-165); the bridge merely declined the unknown rule to the `other =>`
internal-error arm, dragging any file containing `import(x)` to WHITESPACE_ONLY.

**Fix (CLOC12.169 PR2):** a new `convert_dynamic_import` and a `"dynamic_import"`
dispatch arm. Unlike the atomic `import.meta` leaf (gap-169), this is a
**compound** single-operand node: the converter extracts the sole Node child (the
module-specifier expression, via `node_children`), converts it recursively with
`convert_expression`, and wraps it in `ImportExpression { cv, source }`. The
recursion means a fold inside the specifier (`import("a" + "b")` → `import("ab")`)
propagates. The `import` token's `cv` becomes the node's provenance, mirroring
`convert_import_meta`. 3 bridge unit tests plus the closurec e2e diff fixture
`tests/diff/simple-importexpr/` (`f(import("m"), 1 + 2);` → `f(import("m"),3);`)
prove `import(x)` now flows through the full SIMPLE pipeline. The **PR3**
CodePrinter conformance port follows.


## CLOC12.168 — `ImportMeta` (`import.meta`): atomic node + emit + passes (PR1)

Adds `Expression::ImportMeta { cv }` to `javascript-ast` (0.27.0) — the
`import.meta` module meta-property exposing host-provided metadata about the
current module (canonically `import.meta.url`). It is the direct **sibling of
`NewTarget`**: ESTree models both `new.target` and `import.meta` as
`MetaProperty` nodes, so `import.meta` gets its own atomic leaf variant here
rather than being a member access of an `import` identifier — `import` is a
reserved word and there is no such identifier to access.

It is modelled as its own variant (not `MemberExpression { object: Identifier
"import", property: "meta" }`) for the same reason `NewTarget` / `Super` /
`this` are: the spelling contains reserved words that can never be renamed, so
the renaming passes (`rename`, `rename-globals`, `rename-properties`) must never
touch it. A dedicated variant lets those passes exclude it structurally.

**Emit (`closure-emitter` 0.32.0).** `emit_import_meta` writes the bare
eleven-character spelling `import.meta`. It tags at `PREC_PRIMARY` — the
tightest level, like `this` / `super` / `new.target` — so it never needs
wrapping in any parent (`import.meta.url`, `f(import.meta)`, `import.meta+1` all
print bare) and never forces a paren around an operand (it carries none). The
internal `.meta` is part of the fixed spelling, NOT a member access; a genuine
property read like `import.meta.url` wraps this leaf in an outer
`MemberExpression`.

**Passes (PATCH bumps).** The three rebuild passes (`constant-fold`, `dce`,
`fold-control-flow`) clone the leaf through unchanged like the literals;
`fold-control-flow` also gains an `ImportMeta` arm in its `expression_cv`
accessor. The six traversal passes get a no-op arm (`import.meta` binds/
references no ordinary identifier and has no sub-expression). Atomic node PR1:
node + emit + all nine downstream pass arms land together so the workspace
never breaks.

### gap-169 — bridge declines `import.meta` (`ImportMeta`) — **RESOLVED (javascript-parser 0.32.0, CLOC12.168 PR2)**

The `javascript-parser` bridge previously did not convert `import.meta` to
`Expression::ImportMeta`, so any file containing it dropped to WHITESPACE_ONLY.
The atomic node PR (CLOC12.168 PR1) landed the node + emit + pass traversals so
the typed AST and every downstream pass could already represent and print an
`import.meta`.

**Probe divergence from the recipe.** The grammar was expected (by analogy with
`new.target`, gap-168) to fold `import.meta` into a `member_expression` with an
`import` token base. A probe showed otherwise: the grammar emits a **dedicated
`import_meta` leaf** whose children are the three bare tokens `[Token("import"),
Token("."), Token("meta")]` with no Node child — and this rule was *not* listed
in the bridge's expression dispatch, so it fell through to the `other =>`
internal-error arm (worse than a graceful decline). Note the pre-existing
`import_meta_expression` decline arm was a **different, never-emitted rule
name** — dead code; PR2 removed it. **Like `new.target` and unlike `await`
(gap-165), no grammar work was required** — the token was already produced.

**Fix (CLOC12.168 PR2):** a new `convert_import_meta` and an `"import_meta"`
dispatch arm lower the leaf to `Expression::ImportMeta` (the `import` token's
`cv` becomes provenance; the `.meta` is spelling, not a member access). 3 new
bridge unit tests (`import.meta;` → `ImportMeta`; `import.meta.url;` → member
access whose object is `ImportMeta`; `f(import.meta);` in argument position)
plus the closurec end-to-end diff fixture `tests/diff/simple-importmeta/`
(`f(import.meta, 1 + 2);` → `f(import.meta,3);` at SIMPLE — `import.meta`
round-trips and the argument folds, proving the pipeline ran rather than
falling back to WHITESPACE_ONLY). The **PR3** CodePrinter conformance port is a
separate slice.


## CLOC12.167 — `NewTarget` (`new.target`): atomic node + emit + passes (PR1)

Adds `Expression::NewTarget { cv }` to `javascript-ast` (0.26.0) — the
`new.target` meta-property, the reserved-word **leaf** sibling of `this` /
`super`. It reads the constructor a function was invoked with (`undefined` for
a plain call; the constructor for a `new` call). Like `this` / `super` it is
the simplest possible node: no operand, no axis, the same shape as `Super` /
`ThisExpression`.

Although `new.target` is spelled with **two tokens plus a dot** in source, it
is semantically an *atomic* meta-property — the `.` is part of the spelling,
not a member access — so it is modelled as its own leaf variant rather than a
`MemberExpression`. It can never be a variable name, so the renaming passes
must exclude it structurally.

**Emit (`closure-emitter` 0.31.0).** `emit_new_target` writes the literal
ten-character spelling `new.target`. `new.target` tags at `PREC_PRIMARY` — the
tightest level, like `this` — so it never needs wrapping in any parent
(`new.target.x`, `f(new.target)`, `new.target+1` all print bare) and never
forces a paren around an operand (it has none).

**Passes (PATCH bumps).** The three rebuild passes (`constant-fold`, `dce`,
`fold-control-flow`) clone the leaf through unchanged like the literals;
`fold-control-flow` also gains a `NewTarget` arm in its `expression_cv`
accessor. The six traversal passes get a no-op arm (`new.target` binds/
references no ordinary identifier and has no sub-expression). Atomic node PR1:
node + emit + all nine downstream pass arms land together so the workspace
never breaks.

### gap-168 — bridge declines `new.target` (`NewTarget`) — RESOLVED (CLOC12.167 PR2, javascript-parser 0.31.0)

The `javascript-parser` bridge now converts `new.target` to
`Ok(Expression::NewTarget(NewTarget { cv: … }))` in `convert_member_expression`
instead of declining it (which dragged any file containing `new.target` to
WHITESPACE_ONLY). Like `this` (gap-166) and `super` (gap-167) — and **unlike
`await` (gap-165)** — `new.target` was already parseable, so PR2 was a pure
bridge slice with **no grammar work**.

**Divergence from this spec's earlier assumption (corrected by probe).** The
prior draft claimed a dedicated `new_target_expression` grammar rule that the
bridge reached and declined. Probing the actual parse tree showed otherwise:
`new.target` lowers to a **`member_expression` whose children are three bare
tokens** `[Token("new"), Token("."), Token("target")]` with **no Node child**
(the standalone `new_target_expression` rule is not the path the parser takes
for `new.target`). The fix therefore mirrors the `super`-token base rather than
a top-level rule decline: a `new_target` flag (`nodes.is_empty() &&
has_token("new") && has_token("target")`) relaxes the empty-nodes guard and
returns the atomic `NewTarget` leaf, distinguishing the meta-property from the
argumented `new X(args)` constructor (which always carries a Node callee) on
`nodes.is_empty()`. The `new` token's `cv` becomes the node's provenance — the
`.` is fixed spelling, not a member access. Verified end-to-end: bridge tests
`new_target_meta_property` / `new_target_in_function_return` /
`new_target_as_member_object` (147 pass), and closurec e2e fixture
`tests/diff/simple-newtarget/` (`f(new.target, 1 + 2);` → `f(new.target,3);`,
closurec 0.234.6).


## CLOC12.166 — `Super` (`super`): atomic node + emit + passes (PR1)

Adds `Expression::Super { cv }` to `javascript-ast` (0.25.0) — the `super`
keyword, the reserved-word **leaf** sibling of `this`. `super` names the home
object's prototype (in a method: `super.m()`, `super[k]`) or the superclass
constructor (in a derived constructor: `super(a, b)`). Like `this`, it is the
simplest possible node: no operand, no axis, the same shape as `ThisExpression`
/ `NullLiteral`.

It is modelled as its own variant rather than `Identifier { name: "super" }`
for the same reason as `this`: `super` is a reserved word, can never be a
variable name, and the renaming passes must exclude it structurally. `super`
is *syntactically* restricted — legal only as a member-object or call-callee
inside a method / derived constructor — but that is the **parser's** concern;
the AST and emitter treat it as a plain leaf primary and print it wherever it
is placed.

**Naming — divergence from CLOC02 spec.** CLOC02 §Expression pencilled in
`Super(SuperExpression)`. The node is named **`Super`** (struct `Super`) to
match ESTree's node type exactly (ESTree uses bare `"Super"` — note the
asymmetry with `"ThisExpression"`). The CLOC02 spec line is updated to
`Super(Super)` to match the implementation.

**Emit (`closure-emitter` 0.30.0).** `emit_super` writes the bare five-character
keyword. `super` tags at `PREC_PRIMARY` — the tightest level, exactly like
`this` — so it never needs wrapping in any parent (`super.x`, `super()`,
`super.m()` all print bare) and never forces a paren around an operand.

**Passes (PATCH bumps).** The three rebuild passes (`constant-fold`, `dce`,
`fold-control-flow`) clone the leaf through unchanged like the literals;
`fold-control-flow` also gains a `Super` arm in its `expression_cv` accessor.
The six traversal passes get a no-op arm (`super` binds/references no ordinary
identifier and has no sub-expression). Atomic node PR1: node + emit + all nine
downstream pass arms land together so the workspace never breaks.

### gap-167 — bridge declines `super` (`Super`) — **RESOLVED (javascript-parser 0.30.0, CLOC12.166 PR2)**

The `javascript-parser` bridge previously returned `UnsupportedSyntax { rule:
"Super" }` for the `super` primary, so any file containing `super` dropped to
WHITESPACE_ONLY. Like `this` (gap-166) and **unlike `await` (gap-165)**,
`super` was already parseable — the grammar produces the token the bridge
*reaches* and explicitly declined — so PR2 was a pure bridge slice with **no
grammar work required**.

**Fix (CLOC12.166 PR2):** the shape differs from `this`. Where `this` is a
`primary_expression` handled in `convert_primary_token`, the grammar emits
`super` as a bare *token* directly among the `member_expression` children (not
wrapped in a `primary_expression` Node). `convert_member_expression` therefore
gains a `super` base-branch parallel to the `new` branch — the `super` token
becomes the base `Expression::Super` and the existing suffix-fold loop composes
`.NAME` / `[expr]` / call arguments onto it (`super.m`, `super[k]`,
`super.m(a)`). The `nodes.is_empty()` guard is relaxed for the `super`-token
base (it has no Node child), and a lone `super` returns `Super` directly. 4 new
bridge unit tests (`super.x`, `super[k]`, `super.m(1 + 2)`) plus the closurec
end-to-end diff fixture `tests/diff/simple-super/` (`super.f(1 + 2);` →
`super.f(3);` at SIMPLE — the `super` receiver round-trips and the argument
folds, proving the pipeline ran rather than falling back to WHITESPACE_ONLY).
The **PR3** CodePrinter conformance port follows.


## CLOC12.165 — `ThisExpression` (`this`): atomic node + emit + passes (PR1)

Adds `Expression::ThisExpression { cv }` to `javascript-ast` (0.24.0) — the
`this` keyword, a reserved-word **leaf** primary that reads the current
execution context's `this` binding. It is the simplest node in the recent
run: no operand, no axis, the same shape as `NullLiteral` / `UndefinedLiteral`.

It is modelled as its own variant rather than `Identifier { name: "this" }`
because `this` is a reserved word — it can never be a variable name, so the
renaming passes (`rename`, `rename-globals`, `rename-properties`) must never
touch it. A dedicated variant lets those passes exclude it structurally instead
of string-comparing every identifier name.

**Emit (`closure-emitter` 0.29.0).** `emit_this` writes the bare four-character
keyword. `this` tags at `PREC_PRIMARY` — the tightest level — so it never needs
wrapping in any parent (`this.x`, `this()`, `f(this)`, `this+1` all print bare)
and never forces a paren around an operand.

**Passes (PATCH bumps).** The three rebuild passes (`constant-fold`, `dce`,
`fold-control-flow`) clone the leaf through unchanged like the literals;
`fold-control-flow` also gains a `ThisExpression` arm in its `expression_cv`
accessor. The six traversal passes get a no-op arm (`this` binds/references no
identifier and has no sub-expression). In `inline`, `this` is deliberately left
OUT of the trivial-/pure-expression predicates: `this` is bound at the *call
site*, so treating it as a freely-substitutable primary would be unsound; the
inliner handles it conservatively.

### gap-166 — bridge declines `this` (`ThisExpression`) — **RESOLVED (javascript-parser 0.29.0, CLOC12.165 PR2)**

The `javascript-parser` bridge previously returned `UnsupportedSyntax { rule:
"ThisExpression" }` for the `this` primary token, so any file containing `this`
dropped to WHITESPACE_ONLY. The atomic node PR (CLOC12.165 PR1) landed the node
+ emit + pass traversals so the typed AST and every downstream pass could
already represent and print a `this`.

**Unlike `await` (gap-165), `this` was already parseable.** The grammar produces
a `this` node that the bridge *reaches* and explicitly declined — so PR2 was a
pure bridge slice with **no grammar work required**.

**Fix (CLOC12.165 PR2):** `convert_primary_token`'s `"this"` arm now returns
`Ok(Expression::ThisExpression(ThisExpression { cv: t.cv.clone() }))` (mirroring
the `null` / `undefined` / `true` / `false` keyword arms). 2 new bridge unit
tests (`this;` → `ThisExpression`; `this.x;` → member access whose object is a
`ThisExpression`) plus the closurec end-to-end diff fixture
`tests/diff/simple-this/` (`this.f(1 + 2);` → `this.f(3);` at SIMPLE — the
`this` receiver round-trips and the argument folds, proving the pipeline ran
rather than falling back to a verbatim WHITESPACE_ONLY pass). The **PR3
conformance port** (`code_printer_this_test.rs`) exercises the emitter via
hand-constructed AST separately.
