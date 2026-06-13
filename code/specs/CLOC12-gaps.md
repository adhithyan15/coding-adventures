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
- **Resolution:** `format_js_number` now computes both decimal and exponential forms for finite non-zero numbers and returns the shorter (ties → decimal). `format_exponential_uppercase` wraps Rust's `{:e}` formatter and uppercases the `e`. Examples: `1000000000` → `1E9`, `5000000` → `5E6`, `1.5e-10` → `1.5E-10`. Small integers and decimals stay decimal. NaN/Infinity unchanged. The `test_number_formatting_shortest_form` ignored placeholder in `tests/upstream/code_printer_test.rs` stays — to be re-port'd with real upstream `assertPrint` lines in a follow-up; the underlying emitter behaviour is in place.

### gap-026 — String quote-choice optimisation not implemented

- **Status:** RESOLVED in CLOC12.11
- **Upstream test:** `CodePrinterTest` quote-choice lines
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** Added `choose_quote_and_escape(value)` + `escape_str_sq` helpers in `closure-emitter/src/lib.rs`. `emit_string` now picks single-quote when value contains strictly more `"` than `'` (each saved `\"` is one fewer escape); double-quote otherwise (canonical, ties picked toward double). `ascii_only` mode still always uses double — that's upstream's own invariant. Six new inline tests cover all branches. The `test_string_quote_choice_minimises_escapes` ignored placeholder in the upstream port file will be re-port'd with real upstream `assertPrint` cases in a follow-up.

### gap-027 — Precedence-aware paren insertion not implemented

- **Status:** RESOLVED in CLOC12.10 (incidental, via the same fix as gap-024)
- **Upstream test:** `CodePrinterTest` operator-precedence lines (e.g. `a*(b+c)` keeps inner parens)
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Resolution:** The precedence ladder added for gap-024 covers this directly. `emit_expression_inner(e, parent_prec)` checks `expr_prec(e) < parent_prec` and inserts parens when so — which means `a * (b + c)` correctly keeps the inner parens because `+` (prec 11) < `*` (prec 12). The `test_operator_precedence_inserts_inner_parens` ignored placeholder will be re-port'd with real upstream test cases in a follow-up; the emitter machinery is already in place.

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

- **Status:** OPEN — newly discovered by CLOC14.9. **Lexer-level gap** (NOT a whitespace_only bug).
- **Upstream byte-identity test:** `minify_template_subst` and `minify_tagged_subst` seed fixtures.
- **Why it fails:** Our JavaScript lexer raises `LexerError: Unexpected sequence '` `` ` `` `'` when it encounters the closing backtick of a template like `` `hello ${name}` ``. The lexer currently treats template literals as a single atomic token (`` `…` ``), but substitution templates require multi-segment lexing per §12.8.6:
  - `TEMPLATE_HEAD` — `` `…${ ``
  - `TEMPLATE_MIDDLE` — `}…${`
  - `TEMPLATE_TAIL` — `}…` ``
  - And the embedded expression is regular tokens between the head and tail/middle.
- **What it needs:** Extend the JavaScript lexer's template-literal handling to emit the head/middle/tail variants and re-enter expression-tokenisation mode between segments. Once the lexer emits these correctly, the whitespace_only pass needs minimal-or-no changes — the segments are emitted verbatim along with the substitution expression tokens.
- **Cross-cutting:** Closing this gap also unblocks template-substitution support in other downstream passes (AST emitter, constant folding, etc.). The grammar file (`code/grammars/javascript.grammar`) and `javascript-lexer` crate are the implementation surface.

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

- **Status:** OPEN — newly discovered by CLOC14.13.
- **Upstream byte-identity test:** `minify_for_await_of` seed fixture (general repro: `function f(){for(var v of a){a;}}`).
- **Input:** `function f(){for(var v of a){a;}}`
- **Upstream:** `function f(){for(var v of a)a};`
- **closurec:** `function f(){for(var v of a)a;};`
- **Why it fails:** gap-032's single-stmt block-flatten unwraps `{a;}` → `a;`, but the resulting `;` between the for-body's last statement and the outer function-`}` is NOT dropped by Rule A. Rule A drops source `;` before `}`, but this `;` survives. Probable cause: gap-032's flatten happens after Rule A's pass (or in a different pipeline stage), so Rule A doesn't see this position again. Confirmed NOT specific to `for-await-of` — reproduces with plain `for-of`, `for-in`, and likely `if`/`while`/`do` flattened bodies.
- **What it needs:** Either (1) re-run Rule A after gap-032 flatten, or (2) gap-032 itself peeks the next-after token; if it's `}`, drop the trailing `;` from the flattened content. Approach (2) is more local — the flatten knows exactly what it's emitting and what comes after.

### gap-049 — flattened single-stmt for-body keeps trailing `;` before `}`

- **Status:** **RESOLVED** in CLOC12.56 (PR pending). `minify_for_await_of` flipped IGNORED → PASS. Also tightened `gap032_nested_if_does_not_flatten` expectation (improvement: `if(x){if(y)a()}` instead of `if(x){if(y)a();}` — one byte shorter, still valid JS).
- **Upstream byte-identity test:** `minify_for_await_of` seed fixture (general repro: `function f(){for(var v of a){a;}}`).
- **Why it failed:** gap-032's flatten emitted content `(idx+1)..close_idx` verbatim, which always includes the trailing `;`. When the next token after the closing `}` was itself a `}`, that `;` became redundant — Rule A would have dropped a source `;` at that position, but Rule A doesn't re-scan pre-emitted content.
- **Fix:** In gap-032's eligible branch, peek `kept.get(close_idx + 1)`. If it equals `}`, set emit_end to `close_idx - 1` (exclude the trailing `;` from the inline emission). The eligibility check already verified `last_before_close == ";"`, so this index is always the redundant `;`.

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

- **Status:** OPEN (discovered CLOC14.38). `minify_precedence_operand` ignored.
- **Input:** `var x=a==(b+c);` → **Upstream:** `var x=a==b+c;` (also `a||(b&&c)` → `a||b&&c`, `(a*b)+c` → `a*b+c`).
- **What it needs:** The fuller version of gap-077/078, which only strip an ATOMIC (self-delimiting) operand. Upstream also strips when the parenthesised operand's lowest-precedence operator binds *at least as tightly* as the outer operator (so removing the parens does not change grouping): `+` binds tighter than `==`, `&&` tighter than `||`, `*` tighter than `+`. Needs an operator-precedence table and an "outer op" lookup on both the left (`(a*b)+c`) and right (`a==(b+c)`) operand sides. Must still KEEP `a*(b+c)`, `a-(b-c)` (associativity/precedence would change).

### gap-084 — nested double-paren full strip around var-init RHS

- **Status:** RESOLVED in CLOC12.90. `minify_double_paren_varinit` enforced.
- **Input:** `var x=((a));` → **Upstream:** `var x=a;` (closurec previously reached `var x=(a);` — one layer short). Also `(((a)))` → `a`, `((a+b))` → `a+b`.
- **What it needed:** the gap-053 var-init elision strips only the OUTERMOST `=(…)` layer per pass and then advances past it (`i = close_idx + 1`), so the paren it exposes is never re-examined — `((a))` peels to `(a)` and stops. (The earlier theory blamed gap-062 double-paren collapse, but the var-init case is purely gap-053's single-layer-per-pass behaviour.)
- **CLOC12.90 resolution:** Wrapped the gap-053 var-init elision in a **fixpoint loop** — repeat the whole pass until an iteration drops nothing. This peels every redundant layer (`((a))` → `(a)` → `a`; `(((a)))` → … → `a`; `((a+b))` → `(a+b)` → `a+b`, each layer being the whole RHS) while the existing top-level-comma guard still halts at the load-bearing layer (`((a,b))` → `(a,b)`; a bare `a,b` RHS would split into two declarators). Termination is guaranteed: each iteration removes ≥2 tokens or makes no change and breaks. **Residual (deferred):** `((a))+b` → upstream `a+b` (closurec reaches `(a)+b`) — here gap-053 never fires (the RHS is not *just* the parens), and the paren gap-062 exposes would need gap-077 to re-run; and `if((a))b();` → `if(a)b();` (the if-condition double-paren is a different anchor). Both leave valid output. 2 unit tests + the byte-identity fixture.

### gap-085 — fractional / over-u128 float shortest-form (residual of gap-082)

- **Status:** OPEN (split out of gap-082 by CLOC12.91). No dedicated fixture yet.
- **Input:** `var x=0.5;` → **Upstream:** `var x=.5;` (also `1e-5` → `1E-5`, `0.0001` → `1E-4`, `1.50` → `1.5`, and over-range `1e100` → `1E100`).
- **What it needs:** gap-082 closed the **integer-valued** decimal float/scientific subset (any literal whose exact value is a non-negative integer ≤ u128::MAX, recovered by `decimal_float_as_u128`). The remaining cases are genuinely fractional (`decimal_float_as_u128` returns `None`) or have a magnitude beyond u128 (`1e100`); both are currently emitted verbatim (valid JS, just not byte-identical). Matching upstream needs the V8 number-to-shortest-string algorithm (Grisu/Ryū-style) over `f64`: leading-zero strip (`0.5` → `.5`), trailing-zero strip (`1.50` → `1.5`), and the decimal-vs-exponential cut-over with negative exponents (`0.0001` → `1E-4`). Separately, the trailing-bare-dot form `5.`/`50.` is a **lexer** tokenisation issue (the lexer splits `5.` into NUMBER `5` + DOT `.`), not a number-formatter gap, and should be tracked on the lexer side.

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

- **Status:** OPEN (discovered CLOC14.40). `minify_str_codepoint_esc` / `minify_str_hex_esc` / `minify_str_null_esc` ignored. **HIGH PRIORITY — this corrupts the output string value, not merely byte-identity.**
- **Input:** `var s="\x41";` → **Upstream:** `var s="A";` (also `"\u{1F600}"` → `"😀"`, `"\0"` → `"\x00"`). closurec emits `"x41"` / `"u{1F600}"` / `"0"` — the **backslash is DROPPED**, so the string now holds the literal escape text instead of the intended character.
- **What it needs:** `push_quoted_string_content` (and/or the lexer's string-token decoding) explicitly handles `\n`/`\t`/`\\`/`\"` but DROPS the backslash of every other escape (`\xNN`, `\u{…}`, `\uNNNN` in some forms, `\0`, legacy octal). The fix must preserve (decode + re-escape, or at minimum pass through verbatim) every ECMAScript string escape so the value is never corrupted. Matching upstream byte-for-byte additionally requires decoding to the code point and re-emitting in upstream's canonical form (`\x41` → `A`, `\u{1F600}` → the surrogate pair `😀`, `\0` → `\x00`). The CORRECTNESS half (don't drop the backslash) should land first.

### gap-091 — BigInt radix literal → decimal (WHITESPACE_ONLY)

- **Status:** RESOLVED in CLOC12.96. `minify_bigint_hex` / `minify_bigint_bin` enforced.
- **Input:** `var x=0xFFn;` → **Upstream:** `var x=255n;` (also `0o17n` → `15n`, `0b101n` → `5n`).
- **What it needs:** the BigInt branch of `normalize_number_value` currently only strips the `_` separator (gap-048). Upstream converts a radix BigInt to its shortest decimal form, exactly as gap-038 does for non-BigInt hex/oct/bin. Extend the BigInt branch to parse the `0x`/`0o`/`0b` body and re-emit `{decimal}n`. Small values fit in u128; very large BigInts would need real bigint arithmetic (residual).
- **CLOC12.96 resolution:** Extended the BigInt branch of `normalize_number_value`: after stripping `_` separators (gap-048), the body is parsed as a `0x`/`0o`/`0b` radix literal into a `u128` (`from_str_radix`) and re-emitted as `{decimal}n`. A decimal BigInt body has no radix prefix and falls through unchanged (already shortest — `255n` stays `255n`); an over-`u128` magnitude (e.g. a 140-bit `0xFF…FFn`) leaves the literal verbatim (real bigint arithmetic is a residual). JAR-verified across `0xFFn`/`0XFFn`→`255n`, `0o17n`→`15n`, `0b101n`→`5n`, `0x1_FFn`→`511n` (separator+radix), `0n`→`0n`. 2 dedicated unit tests + the two byte-identity fixtures; three pre-existing gap-038/048 tests that asserted the deferred radix-BigInt behavior (`0xfn` unchanged, `0x1FFFn` unchanged) were updated to the now-correct decimal forms.

### gap-092 — division mis-lexed as regex (WHITESPACE_ONLY)

- **Status:** OPEN (discovered CLOC14.40). `minify_regex_div` ignored.
- **Input:** `var x=a/b/c;` → **Upstream:** `var x=a/b/c;` (closurec emits `a /b/ c`).
- **What it needs:** the JavaScript lexer treats the `/b/` in `a/b/c` as a REGEX literal rather than two DIVISION operators, and the re-stitcher then adds separating spaces around the "regex". Regex-vs-division disambiguation requires knowing whether the preceding token ends an expression (then `/` is division) or not (then `/` may start a regex) — a lexer-level concern. The output stays valid JS (same grouping), so this is byte-identity only, not a correctness bug.

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

- **Status:** OPEN (discovered CLOC14.41). `minify_chained_new` ignored.
- **Input:** `new new A;` → **Upstream:** `new (new A);` (closurec leaves `new new A`).
- **What it needs:** upstream wraps the inner `new` of a chained `new new A` to `new (new A)`, disambiguating the inner NewExpression as the outer `new`'s callee. closurec's output `new new A` is valid and equivalent, just not byte-identical — a low-priority normalisation.

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
