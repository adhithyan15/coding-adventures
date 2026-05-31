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

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNullComparison1` (`null OP null` self-relation lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** `try_fold_binary_op` has dedicated branches for `NumericLiteral`/`NumericLiteral`, `StringLiteral`/`StringLiteral`, `BooleanLiteral`/`BooleanLiteral`, but no branch for `NullLiteral`/`NullLiteral`. The binary node falls through and is returned unchanged.
- **What it needs:** A small `NullLiteral`/`NullLiteral` branch returning `Boolean(true)` for `==`/`===`/`<=`/`>=` and `Boolean(false)` for `!=`/`!==`/`<`/`>`. Roughly 10 lines in `try_fold_binary_op`.

### gap-008 — cross-type strict equality fold (`Number === String → false`)

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberStringComparison` (`===`/`!==` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Strict equality between two literals of *different* JS types is `false` by definition (and `!==` is `true`). The current pass only fires same-type branches, so `1 === '1'` falls through and is returned unchanged.
- **What it needs:** A pre-branch in `try_fold_binary_op` that, when both sides are literals of different JS types *and* the operator is `===`/`!==`, returns `Boolean(false)`/`Boolean(true)`. Roughly 15 lines, fully self-contained.

### gap-006 — unary plus / minus on identifiers, plus identifier-arithmetic shape

- **Status:** OPEN
- **Upstream test:** `PeepholeFoldConstantsTest::testNumberNumberComparison` (`+x > +y` `testSame` lines)
- **Ported file:** `closure-pass-constant-fold/tests/upstream/peephole_fold_constants_test.rs`
- **Why it fails:** Upstream's `testSame("+x > +y")` asserts the pass leaves the expression alone (`x` is unknown). Our pass already does the right thing structurally; this gap is mostly the bookkeeping of porting the remaining `testSame` lines that use unary-plus on identifiers.
- **What it needs:** Trivial — extend the ported tests once `gap-005` lands so the batch reflects the full upstream method.
