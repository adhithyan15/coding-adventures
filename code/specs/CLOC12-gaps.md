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

### gap-021 — `BigIntLiteral` not in Phase 1 AST

- **Status:** OPEN
- **Upstream test:** `CodePrinterTest::testBigInt`
- **Ported file:** `closure-emitter/tests/upstream/code_printer_test.rs`
- **Why it fails:** No `Expression::BigIntLiteral` variant — `1n`, `0x4n`, `-5n` etc. have no AST representation in Phase 1.
- **What it needs:** Phase 1.x AST extension: `BigIntLiteral { value: ?, raw: String }`. Then teach the emitter to render the literal with the `n` suffix, including for the negative-literal case.

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

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testStringStringComparison` (`typeof a === typeof a` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream folds `typeof a === typeof a` → `true` and `typeof a !== typeof a` → `false` because the two sub-expressions are *structurally identical*. Our constant-fold pass folds by *value substitution* — it doesn't compare two expressions for structural equality.
- **What it needs:** A new fold rule on `BinaryExpression` with `StrictEq`/`StrictNotEq` that, when neither side is foldable to a literal, tests whether the two sides are syntactically equivalent. If both sides are the same `UnaryExpression { op: TypeOf, argument: <pure expression> }` shape with `argument` being the same identifier, fold to `true`/`false` respectively. Care needed: only fire when the argument is provably side-effect-free (an `Identifier` or another literal). Roughly 30-40 lines in `try_fold_binary_op`.
