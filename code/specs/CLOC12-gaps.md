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

- **Status:** OPEN — discovered by CLOC14.35. `minify_await_paren_elide` ignored.
- **Input:** `await(x)` → `await x`.
- **What it needs:** Same simple-reference operand paren elision as gap-070/071, but for the `await` unary operator. Deferred: `await` is async-context-only, so a naive keyword-anchored elision could mis-handle a non-async `await` used as a plain identifier (`await` is only reserved inside async functions/modules). Needs an async-context guard before it can be added to the keyword set safely.

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
