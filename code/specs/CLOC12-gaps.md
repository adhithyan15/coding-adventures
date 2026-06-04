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

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberStringComparison`, `PeepholeFoldConstantsTest::testStringNumberComparison`
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream folds `1 < '2'` to `true` (string `'2'` coerced via `ToNumber`) and `1 == '2'` to `false` (loose equality, per ES spec §IsLooselyEqual). Our pass leaves mixed-type `==`/`<`/`>` alone.
- **What it needs:** Same shape as gap-003 — implement abstract-equality and abstract-relational-comparison for compile-time constants.

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

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberNumberComparison` (`+x > +y` `testSame` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream's `testSame("+x > +y")` asserts the pass leaves the expression alone (`x` is unknown). Our pass already does the right thing structurally; this gap is mostly the bookkeeping of porting the remaining `testSame` lines that use unary-plus on identifiers.
- **What it needs:** Trivial — extend the ported tests once `gap-005` lands so the batch reflects the full upstream method.

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

- **Status:** RESOLVED in CLOC12.18 (PR pending).
- **Upstream test:** `PeepholeMinimizeConditionsTest::testFoldOneChildBlocks` (`if(x){foo()}else{bar()}` → `x?foo():bar()` lines)
- **Ported file:** `closure-pass-fold-control-flow/tests/upstream/peephole_minimize_conditions_test.rs`
- **Resolution note:** Added a rewrite arm in `fold_if_statement` that fires when the test isn't a literal AND both branches reduce to a single `ExpressionStatement` (recursively unwrapping single-statement `BlockStatement` layers via a `single_expr_stmt` helper). Emits an `ExpressionStatement` wrapping a `ConditionalExpression`. Side-effect-safe because ternary preserves the same evaluation order as if-else (test first, then exactly one branch). `test_fold_one_child_blocks_if_else_to_ternary` un-ignored.

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

- **Status:** OPEN
- **Upstream test:** `CodePrinterTest::testTrailingCommaInArrayAndObjectWithPrettyPrint` and ~6 sibling tests
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Why it fails:** Our emitter doesn't model whether trailing commas are present in `[1,]` / `{a:1,}` — there's no `trailing_comma: bool` flag on the relevant AST nodes, and the emitter doesn't insert / preserve them.
- **What it needs:** AST flag + emitter rule + (optional) pretty-print toggle.

### gap-023 — `VariableDeclaration` round-trip ports deferred

- **Status:** OPEN
- **Upstream test:** Most `assertPrintSame("var x = …")` lines in `CodePrinterTest`
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Why it fails:** Hand-constructing `VariableDeclaration` ASTs (with `VariableDeclarator`s, `id`, `init`, `kind`) for every upstream test is verbose. Deferred to a dedicated future port file that focuses on declaration round-trips.
- **What it needs:** Either a parser bridge (so the upstream `var x = ...` source string can be used directly), or a focused declarations-port-file that pays the verbosity cost. Likely the former.

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

- **Status:** OPEN
- **Upstream test:** `SourceMapGeneratorV3Test::testBasicMapping*`, `testLiteralMappings*`, `testMultilineMapping*`, `testMultiFunctionMapping`, `testGoldenOutput*` (almost the entire upstream file)
- **Ported file:** `closure-source-map/tests/upstream/source_map_generator_v3_test.rs`
- **Why it fails:** Our `SourceMapBuilder` v0.1.0 accumulates raw `(line, column, cv_id)` mappings but the `build()` step produces a `SourceMap` with `mappings: String::new()` — VLQ encoding is documented as v2 work in the crate's source. Upstream tests assert specific VLQ strings like `"A,aAAAA,QAASA,UAAS,EAAG;"` and need the encoder to produce them.
- **What it needs:** Implement the VLQ encoder per the source-map v3 spec. The encoder receives per-token `(generated_line, generated_column)` paired with `cv_id`, resolves each `cv_id` to `(source_index, original_line, original_column)` via the `CVLog`, and emits the standard base64-VLQ delta-encoded `mappings` string. Once it lands, the seven `#[ignore]`-ed ports in `source_map_generator_v3_test.rs` flip to real assertions and we re-port the rest of upstream's `SourceMapGeneratorV3Test`.

### gap-029 — identity-of-typeof-same-identifier fold not implemented

- **Status:** RESOLVED in CLOC12.17 (PR pending).
- **Upstream test:** `PeepholeFoldConstantsTest::testStringStringComparison` (`typeof a === typeof a` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Resolution note:** Added a new structural-equality arm in `try_fold_binary_op` for `StrictEq`/`StrictNotEq` operators where both sides are `UnaryExpression { op: TypeOf, argument: Identifier }` with the same identifier name. Folds to `true`/`false` respectively. Identifier-only because `typeof <undeclared>` is special-cased by ECMAScript §UnaryTypeofExpression to return `"undefined"` rather than throw, so the fold is safe even without declaration-tracking — `typeof x` evaluated twice deterministically produces the same string. Member/call expressions are deliberately NOT folded because they can have side effects that we can't prove are absent without a heavier purity analysis. `test_typeof_identifier_identity_fold` un-ignored.
