# Changelog

All notable changes to the `coding-adventures-closure-pass-constant-fold` crate will be documented in this file.

## [0.13.0] - 2026-06-20

### Added — CLOC20: fold inside `do`/`while`

`fold_tagged` now has a `DoWhileStatement` arm that recurses constant folding into
the loop body and test. Constant expressions inside a do-while body (e.g.
`1 + 2` ⇒ `3`) now fold like anywhere else.

## [0.12.0] - 2026-06-20

### Added — CLOC19: fold inside `try`/`catch`/`finally`

`fold_tagged` now has a `TryStatement` arm that recurses constant folding into
the protected block, the catch handler body, and the finalizer, preserving the
catch `param` verbatim. Constant initializers and expressions inside try/catch
blocks (e.g. `1 + 2` ⇒ `3`) now fold like anywhere else.

## [0.11.0] - 2026-06-19

### Added — CLOC15.D: fold bitwise & shift operators on numeric literals

`try_fold_binary_op` now folds the six integer operators on two numeric
literals, matching ECMAScript's 32-bit semantics exactly:

```js
0xFF & 0x3C   // ⇒ 60
1 << 4 | 2    // ⇒ 18
8 >>> 1       // ⇒ 4
```

- `&` / `|` / `^` — both operands coerced via **`ToInt32`**, result is a
  signed 32-bit integer.
- `<<` / `>>` — left operand `ToInt32`; the shift COUNT is `ToUint32(rhs) &
  31` (the low 5 bits). `>>` is arithmetic (sign-propagating).
- `>>>` — left operand **`ToUint32`**, logical (zero-fill) shift; the result
  is an **unsigned** 32-bit value, so it can exceed `i32::MAX`
  (`-1 >>> 0 ⇒ 4294967295`).

New `to_int32` / `to_uint32` helpers implement the spec coercions (non-finite
and `±0` → 0; otherwise truncate toward zero and reduce modulo 2³²). Because
the operands are already numeric literals, the coercions are exact and the
fold can never diverge from the runtime value — deterministic and sound.

`FoldedLiteral` gained `#[derive(Debug)]` (for test diagnostics only).

- 3 new tests: `to_int32`/`to_uint32` spec-vectors, the six operators against
  exact JS reference values (incl. fractional-operand coercion, the
  `>= 2³¹ → negative` wrap, 5-bit shift-count masking, arithmetic `>>`, and
  unsigned `>>>`), and an end-to-end pass run confirming the emitter renders a
  `> i32::MAX` result. Full closurec suite + all constant-fold consumers green,
  no fixture churn.

## [0.10.1] - 2026-06-04

### Added — CLOC12.23: gap-006 unary plus / minus on identifier bookkeeping

Closes `gap-006` from the CLOC12 gap tracker. Pure test-only change —
no production code modified.

The pass already does the right thing structurally: `fold_unary` only
folds `+<literal>` / `-<literal>` (the runtime value of `+x` is
unknown when `x` is an identifier), and `try_fold_binary_op` declines
when either side isn't a recognised literal. So `+x > +y` and friends
pass through verbatim. gap-006 was waiting on bookkeeping — port the
upstream `testSame("+x > +y")` / `testSame("+x == +y")` lines from
`PeepholeFoldConstantsTest::testNumberNumberComparison`.

The new `test_same_unary_on_identifier_in_comparison` test in
`peephole_fold_constants_test.rs` pins:

* `+x > +y`, `+x == +y`, `+x === +y` survive unchanged.
* `-x < -y` survives (Negate variant — same reasoning).
* Asymmetric `0 < +x` survives (literal on one side, unary-of-
  identifier on the other — fold must bail because identifier side
  can't be resolved).
* `+x == +x` survives even with the same identifier on both sides:
  `x` could be NaN at runtime, and `NaN == NaN` is `false`.

Upstream test count: 13 → 14.

## [0.10.0] - 2026-06-04

### Added — CLOC12.22: gap-004 Number/String cross-type abstract equality + relational comparison

Closes `gap-004` from the CLOC12 gap tracker. `try_fold_binary_op` now
coerces a String operand against a Number operand by calling a new
conservative subset of ECMAScript §StringToNumber and evaluating the
resulting Number-vs-Number comparison for the loose equality
operators (`==` / `!=`) and the abstract relational operators
(`<` / `<=` / `>` / `>=`).

Worked examples (upstream-pinned):

* `1 < '2'`  → `true`   (string coerced to 2, then 1 < 2)
* `1 == '2'` → `false`  (loose equality: ToNumber('2') === 2)
* `'2' < 1`  → `false`  (order preserved — NOT swapped to `1 < 2`)
* `'1' == 1` → `true`   (symmetric, string-on-left)
* `1.5 == '1.5'` → `true`

What the new `js_string_to_number_strict` helper recognises:

1. **Empty / ASCII-whitespace-only** → `0.0` (per §StringNumericValue).
2. **`Infinity` / `+Infinity` / `-Infinity`** (case-sensitive, after trim).
3. **Decimal-style numeric literals** — `[+-]?\d*(\.\d*)?([eE][+-]?\d+)?` with at least one digit, lone signs/dots rejected.

What it **does not** handle (deliberate follow-ups; returning `None` bails the fold soundly):

* Hex / binary / octal prefixes (`0x...`, `0b...`, `0o...`).
* Non-ASCII JS WhiteSpace (NBSP, ZWNBSP, U+2028, U+2029, ...).
* Strings that evaluate to NaN per spec (e.g. `"hi"`) — folding these
  to `false` for `==` (or `true` for `!=`) is a future optimisation.

Strict equality on Number/String is untouched — gap-008's branch
already returns `false` / `true` and runs after this one is gated out
by `matches!(op, Eq | NotEq | Lt | LtEq | Gt | GtEq)`.

Tests:

* upstream test `test_number_string_comparison_literal_lines`
  un-ignored — was the canonical pin for this gap.
* 8 new inline unit tests cover: (1) the helper's recognised decimal
  cases, (2) explicit Infinity, (3) the conservative-bail set (hex,
  non-numeric, lone-sign, malformed exponent), (4) the upstream
  cases, (5) order-preservation and symmetry, (6) a full truth table
  for both `1 OP '2'` and `1.5 OP '1.5'`, (7) the gap-008 strict
  regression, and (8) the conservative-bail behaviour on `'hi'`.

Touched pre-existing test: `mixed_type_loose_equality_not_folded`
was asserting the old "don't fold mixed-type comparisons" sound
default, which gap-004 has now narrowed to "don't fold when the
string is unrecognisable". The test was renamed to
`mixed_type_loose_equality_with_unrecognised_string_not_folded` and
its example changed from `1 == "1"` (now folds to `true`) to
`1 == "hi"` (still bails, original intent preserved).

No changes to public API or AST surface.

## [0.9.0] - 2026-06-02

### Added — CLOC12.21: gap-003 `null == <primitive>` cross-type loose-equality fold

Closes `gap-003` from the CLOC12 gap tracker. `try_fold_binary_op` now
implements the `null`-side branch of the ECMAScript abstract-equality
algorithm (§IsLooselyEqual) for compile-time-known partner literals.

What's folded:

* `null == X` and `X == null` where `X` is any non-null primitive
  literal (`number`, `string`, `boolean`, `bigint`, or `undefined`).
* The result is `true` iff the partner is `undefined` (the spec
  hard-codes `null == undefined → true`); every other partner is `false`.
* `null != X` and `X != null` fold to the boolean negation.

Truth table:

```
partner          ==     !=
---------------+------+------
null           | true | false   (already covered by gap-007 path)
undefined      | true | false
number         | false| true
string         | false| true
boolean        | false| true
bigint         | false| true
```

Unsoundness guard: if the partner side is an `Identifier` (or anything
non-literal we can't statically classify), the fold bails out. The
identifier's runtime value could itself be `null`/`undefined`, and
folding to a concrete boolean would change observable behaviour.

Ordering: the new branch runs *after* the existing null/null branch
(gap-007) — so by the time we reach it, at most one side is a
NullLiteral — and *before* the cross-type strict-equality branch
(gap-008), which is unaffected because that branch only fires on
`===`/`!==`. A regression test in the inline tests pins gap-008's
behaviour for `null === 0` / `null !== 0`.

Tests:

* `peephole_fold_constants_test::test_null_comparison_1_loose_against_other_types`
  is un-ignored (was `#[ignore = "blocked on gap-003"]`).
* 6 new inline unit tests cover both directions, the `!=` complement,
  the `null == undefined → true` special case, the identifier
  unsoundness guard, and the gap-008 regression check.

No changes to public API or AST surface.

## [0.8.0] - 2026-06-02

### Added — CLOC12.20: gap-002 `void <pure-literal>` → `undefined` fold

Closes `gap-002` from the CLOC12 gap tracker. `UnaryExpression { operator: Void, argument: <primitive-literal> }` now folds to `UndefinedLiteral`. The canonical case `void 0` (a Closure-Compiler-style synonym for `undefined`) is now resolved.

What's folded:

- `void <NumericLiteral>` → `undefined`
- `void <StringLiteral>` → `undefined`
- `void <BooleanLiteral>` → `undefined`
- `void <NullLiteral>` → `undefined`
- `void <BigIntLiteral>` → `undefined`
- `void <UndefinedLiteral>` → `undefined` (idempotent)

What's deliberately NOT folded:

- `void <Identifier>` — the identifier could refer to a function/getter with side effects.
- `void <CallExpression>` — same, the call has observable side effects.
- `void <MemberExpression>` — property accesses can trigger getters / proxies.
- `void <BinaryExpression>` / etc. — recurses through fold; if the inner folds to a primitive literal, the void rule fires on the next iteration.

Soundness: the general rule `void <expr> → undefined` only holds when `<expr>` has no observable side effects. By restricting to primitive literals, we have a strict subset that's *always* sound. Closes the test surface for `testUndefinedComparison2` from the upstream Closure test suite.

### Implementation

- `FoldedLiteral` enum: new `Undefined` variant. Stamp + label helpers updated to handle it.
- `fn fold_unary` `UnaryOperator::Void` arm: matches the 6 primitive-literal variants, returns `Some(FoldedLiteral::Undefined)`; everything else falls through to `None`.
- `use coding_adventures_javascript_ast::UndefinedLiteral` added to the imports.

### Tests

- `tests/upstream/peephole_fold_constants_test.rs::test_undefined_comparison_2`: un-ignored. 4 assertions (`void 0`, `void 1`, `void "x"`, `void undefined`).

Before this PR:
- Total upstream tests: 14, passing: 10, ignored: 4 (gap-001, gap-002, gap-003, gap-004).

After:
- Total upstream tests: 14, passing: 11, ignored: 3 (gap-001, gap-003, gap-004).

The pending gaps (gap-003 cross-type null comparison, gap-004 abstract-equality / abstract-comparison) are independent of this PR — they need the abstract-equality algorithm implemented, which is a separate body of work.

### Bumped 0.7.1 → 0.8.0

`fold` API and CV-stamping semantics unchanged. Version bump reflects the new fold rule (closes one observable upstream test parity gap).

## [0.7.1] - 2026-06-01

### Added — CLOC12.16: typeof `UndefinedLiteral` folds to `"undefined"`

The constant-fold pass gained three `Expression::UndefinedLiteral`
arms so it compiles against the new `javascript-ast 0.6.0` AST:

1. Leaf passthrough — undefined is itself the folded form.
2. `js_literal_type` returns `"undefined"` so the strict-equality
   fold knows `undefined === <other type>` is `false`.
3. `UnaryOperator::TypeOf` over an `UndefinedLiteral` folds to
   `"undefined"`. This closes the final hole in CLOC12.09's
   typeof-literal fold table.

## [0.7.0] - 2026-06-01

### Changed — CLOC12.15 rebase: handle new `BigIntLiteral` Expression variant

The constant-fold pass gained `Expression::BigIntLiteral` arms in
three places so it compiles against the new `javascript-ast 0.5.0`
AST:

1. Leaf passthrough — a `BigIntLiteral` is already in folded form,
   no children to recurse into.
2. `js_literal_type` returns `"bigint"` so the strict-equality
   fold knows two bigint literals share a type tag with each other
   but not with `NumericLiteral` / `StringLiteral`.
3. `UnaryOperator::TypeOf` over a `BigIntLiteral` folds to
   `"bigint"` (the ECMAScript-correct typeof result).

Bigint arithmetic folding (`1n + 2n` → `3n`) is **not** implemented —
it would require a bigint runtime in the pass crate, which is out
of scope for CLOC12.15. The literal is itself the folded form.

Bumped to 0.7.0 (rather than 0.5.3 originally planned) because this
PR was rebased on top of CLOC12.17 (0.6.0, already on main) — both
landings are additive, so a single fresh minor captures the union.

## [0.6.0] - 2026-06-01

### Added — CLOC12.17: typeof-identity fold (closes gap-029)

Adds a new structural-equality arm in `try_fold_binary_op` that
recognises `typeof <Identifier> === typeof <same Identifier>` and
folds to `true`; the `!==` form folds to `false`.

Truth table:

| Input                       | Output      | Why                          |
|-----------------------------|-------------|------------------------------|
| `typeof a === typeof a`     | `true`      | identical sub-expressions    |
| `typeof a !== typeof a`     | `false`     | identical sub-expressions    |
| `typeof a === typeof b`     | unchanged   | different identifier names   |
| `typeof a == typeof a`      | unchanged   | only strict ops are folded   |

**Safety:** ECMAScript §UnaryTypeofExpression special-cases
`typeof <undeclared-identifier>` to return the string `"undefined"`
instead of throwing a ReferenceError, so even when the binding
doesn't exist, evaluating `typeof x` twice produces the same string
both times. This makes the fold sound regardless of whether the
identifier resolves to a real binding.

The fold deliberately fires only on `Identifier` arguments — not
on member/call expressions — because those can have observable
side effects (getter invocation, function call) that we can't
prove are absent without a heavier purity analysis.

Un-ignores `test_typeof_identifier_identity_fold` in the upstream
test port (`tests/upstream/peephole_fold_constants_test.rs`).

## [0.5.2] - 2026-06-01

### Changed — CLOC12.14: handle new `ThrowStatement` variant

The constant-fold pass gained a `TaggedStatement::ThrowStatement`
match arm so it compiles against the new `javascript-ast 0.4.0` AST.
Behaviour: fold the argument expression (so `throw 2+3;` → `throw 5;`),
preserve the throw semantics.

## [0.5.1] - 2026-06-01

### Changed — CLOC12.13: handle new `LabeledStatement` variant

The constant-fold pass gained a `TaggedStatement::LabeledStatement`
match arm so it compiles against the new `javascript-ast 0.3.0` AST.
Behaviour: recurse into the labelled body (so inner constant-folds
reach inside `a: { foo(2+3); }`), preserve the label verbatim. No
new optimisation; this is purely the "stay non-exhaustive-safe"
mechanical change.

## [0.5.0] - 2026-06-01

### Added — CLOC12.09: close gap-005 typeof literal fold

Implements `typeof <primitive literal>` constant-folding per the
ECMAScript §UnaryTypeofExpression table:

| Operand                  | Folded result |
|--------------------------|---------------|
| `NumericLiteral`         | `"number"`    |
| `StringLiteral`          | `"string"`    |
| `BooleanLiteral`         | `"boolean"`   |
| `NullLiteral`            | `"object"`    |

The `NullLiteral → "object"` case preserves the famous JavaScript quirk
where `typeof null === "object"` (a historical bug baked into the spec).

The four remaining `typeof` cases stay deferred:

- `typeof undefined → "undefined"` — gated on gap-001 (no
  `UndefinedLiteral` AST variant yet).
- `typeof <BigIntLiteral> → "bigint"` — gated on gap-021 (no
  `BigIntLiteral` AST variant yet).
- `typeof <function expression> → "function"` — Phase 1.x AST work.
- `typeof <Identifier>` — left alone (identifier may bind to anything at
  runtime; matches upstream `testSame` lines).

### gap-005 → RESOLVED via CLOC12.09

The CLOC12.02 ignored port `test_typeof_lines_from_string_string_comparison`
is replaced by three focused tests:

| New test | Status |
|----------|--------|
| `test_typeof_literal_comparison_folds` | **passing** (`typeof 3 > typeof 4` → `false`) |
| `test_typeof_identifier_is_left_alone` | **passing** (`testSame` shape) |
| `test_typeof_identifier_identity_fold` | `#[ignore]` on **new gap-029** |

### gap-029 — identity-of-typeof-same-identifier fold (NEW)

Upstream folds `typeof a === typeof a` → `true` and
`typeof a !== typeof a` → `false` because the two sub-expressions are
structurally identical. Implementing that requires a *structural
equality* check between operands, which is conceptually distinct from
value-substitution folding. Filed as gap-029 for a future PR.

### Port score (this crate)

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.03   | 7       | 5       |
| **CLOC12.09** | **9** | **5**   |

(Net +2 passing, 0 net change to ignored. The previously-ignored
`test_typeof_lines_from_string_string_comparison` stub got replaced
by 2 new passing tests + 1 new `#[ignore]`-d gap-029 test, so total
test count went 12 → 14.)

### Version bump

`0.4.0` → `0.5.0`.

## [0.4.0] - 2026-05-31

### Added — CLOC12.03: close gap-007 and gap-008

Two small fold-pass body extensions in `try_fold_binary_op`, each
~15 lines, sitting after the existing per-type branches:

**gap-007 — `NullLiteral OP NullLiteral`.** New branch returns the
JS-spec result for every comparison operator on two `null` literals:

```text
null ==  null   →  true
null === null   →  true
null !=  null   →  false
null !== null   →  false
null <   null   →  false   (both coerce to 0; 0 < 0 is false)
null >   null   →  false
null <=  null   →  true
null >=  null   →  true
```

Relational operators run through ECMAScript §IsLessThan, which
calls ToNumber on each side. `ToNumber(null)` is `0`, so the four
relational cases reduce to `0 OP 0`.

**gap-008 — cross-type strict equality.** New branch handles
`StrictEq`/`StrictNotEq` when both operands are literals of
*different* JS types. Per ECMAScript §IsStrictlyEqual, `===` is
`false` for any pair of values with different types, and `!==` is
`true`. So:

```text
1 === "1"          →  false
1 !== "1"          →  true
true === 1         →  false
true !== 1         →  true
null === 0         →  false
"a" === true       →  false
```

This branch fires *after* the same-type branches (numeric/numeric,
string/string, boolean/boolean, null/null), so the only cases left
to handle are literals of recognised-but-different JS types. Loose
`==` is still left alone — that goes through the abstract-equality
algorithm and stays gated by gap-003 / gap-004.

A new internal helper `js_literal_type(&Expression) → Option<&'static str>`
tags each Phase 1 primitive literal with a string discriminator
(`"number"`, `"string"`, `"boolean"`, `"null"`). The tags are
internal — they're not the result of the JS `typeof` operator
(which has its own quirks like `typeof null === "object"`) — but
they're sufficient to decide whether two literals have the same JS
type for the strict-equality fold.

### Test impact

`tests/upstream/peephole_fold_constants_test.rs`:

- `test_null_comparison_1_self_relations` was `#[ignore]`-ed in
  CLOC12.02 with `gap-007` — now passes.
- `test_number_string_strict_equality_lines` was `#[ignore]`-ed in
  CLOC12.02 with `gap-008` — now passes.

Total port score:

|             | passing | ignored |
|-------------|---------|---------|
| CLOC12.02   | 5       | 7       |
| **CLOC12.03** | **7** | **5**   |

`code/specs/CLOC12-gaps.md` updated: `gap-007` and `gap-008` marked
`RESOLVED-in-#NNNN` (PR number filled in once we know it).

### Version bump

`0.3.0` → `0.4.0`.

## [0.3.0] - 2026-05-31

### Added — CLOC12.02: first port of upstream `PeepholeFoldConstantsTest`

This is the **first** ported file under the CLOC12 byte-identical
contract. Establishes the per-crate `tests/upstream/` layout:

- `tests/upstream/UPSTREAM_SHA` — pins
  `google/closure-compiler@5bb35ec1245dc1d3557481e5f8b4db344bcd1e6b`.
- `tests/upstream/ATTRIBUTION.md` — Apache-2.0 attribution per
  CLOC12.01 §5, lists ported files with upstream paths and blob SHAs.
- `tests/upstream/peephole_fold_constants_test.rs` — ports a subset
  of upstream's `PeepholeFoldConstantsTest`:
  - `test_null_comparison_1_self_relations` — `null OP null` for
    `==`, `===`, `!=`, `!==`, `<`, `>`, `>=`, `<=`.
    `#[ignore = "blocked on gap-007"]` (the fold pass has no
    `NullLiteral`/`NullLiteral` branch yet — small, self-contained
    fix).
  - `test_number_number_comparison_literal_lines` — literal-only
    arithmetic comparisons. **Passes today.**
  - `test_string_string_comparison_literal_lines` — literal-only
    string comparisons across `<`, `<=`, `>`, `>=`, `==`, `!=`,
    `===`, `!==`. **Passes today.**
  - `test_number_string_strict_equality_lines` — strict equality
    between Number and String is `false` regardless of values.
    `#[ignore = "blocked on gap-008"]` — the pass falls through its
    same-type branches and returns the binary expression unchanged.
    Trivial small fix queued.
  - `test_basic_number_comparisons` — sanity check of the
    same-type-numeric comparison happy path. **Passes today.**
  - `test_basic_arithmetic_folds` — `2 + 3 = 5`, `"a" + "b" = "ab"`,
    `"x" + 1 = "x1"`, `5 * 4 = 20`, `10 / 2 = 5`, `7 % 3 = 1`,
    `2 ** 8 = 256`. **Passes today.**
  - `test_same_when_either_side_has_an_identifier_subset` —
    `testSame`-style asserts that identifier-bearing comparisons are
    left alone. **Passes today.**
  - `test_undefined_comparison_1` — `#[ignore = "blocked on gap-001"]`.
  - `test_undefined_comparison_2` — `#[ignore = "blocked on gap-002"]`.
  - `test_null_comparison_1_loose_against_other_types` —
    `#[ignore = "blocked on gap-003"]`.
  - `test_number_string_comparison_literal_lines` —
    `#[ignore = "blocked on gap-004"]`.
  - `test_typeof_lines_from_string_string_comparison` —
    `#[ignore = "blocked on gap-005"]`.

Each ignored test cites a `gap-NNN` entry in
`code/specs/CLOC12-gaps.md` describing what's blocked and what
unblocks it. Running `cargo test -- --include-ignored` exercises
the ignored ports too; the gap count is the measurable progress
metric for byte-identical convergence.

### Test scaffolding

The ported file does not depend on a source-string parser bridge
(no such bridge exists yet — `javascript-parser::parse_javascript`
returns the generic `GrammarASTNode`, not our typed `Program`).
Instead, the file constructs typed-AST inputs by hand using the
same literal builders as `closure-pass-constant-fold`'s own inline
tests:

```rust
let input  = b(n(2.0), BinaryOperator::Add, n(3.0));
let expect = n(5.0);
assert_fold(input, expect);
```

When the parser bridge lands (a future CLOC11.* slice), we can
re-port these tests to take the upstream `test("2 + 3", "5")`
source-string form verbatim. Until then, every port both records
the upstream `test(...)` line in a doc-comment and asserts the
same byte output via constructed AST.

### Cargo wiring

Added explicit `[[test]]` entry in `Cargo.toml` pointing at
`tests/upstream/peephole_fold_constants_test.rs` because Cargo's
auto-discovery only picks up `tests/*.rs` one level deep. CLOC12.01
§3 specifies the `tests/upstream/` layout; this is the small price
for keeping ports physically grouped.

### Version bump

`0.2.0` → `0.3.0`.

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
