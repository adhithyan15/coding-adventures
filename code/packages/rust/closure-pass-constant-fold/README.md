# coding-adventures-closure-pass-constant-fold

The **first concrete optimization pass** for the Closure Compiler
clone. Folds compile-time-evaluable expressions per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s canonical
pass set.

## What's here (v1)

- `ConstantFoldPass` implementing the `Pass` trait from
  [`coding-adventures-closure-pass-pipeline`](../closure-pass-pipeline).
- Metadata pinned:
  - `name = "constant-fold"`
  - `iteration_policy = FixedPoint` (folds expose further folds —
    `2 + 3 + 4` becomes `5 + 4` becomes `9` over two iterations)
  - `cost = 2` pass-units (tree walk + small constant work per visit)
  - no `depends_on` or `invalidates` in v1
- `Pass::run` is **identity** in v1: `javascript-ast` ships only
  `Program` / `SourceType` today (per CLOC02 Phase 1), so there's
  nothing to fold. The pass clones the input `Program` unchanged,
  reports `changed = false` and `nodes_touched = 1`, and emits no
  contributions per CLOC03 §"When a pass keeps a node unchanged."

## Why this PR matters even though it's identity

1. Establishes the crate layout future `closure-pass-*` crates mirror.
2. Pins the pass metadata the scheduler reads (name, iteration
   policy, cost).
3. Wires up the CLOC03 contribution-emission path so once the AST
   grows foldable nodes, the integration is in place.

## What's coming

Once `javascript-ast` grows `Statement` / `Expression` variants:

- Number folding: `2 + 3 → 5`, `10 * 4 → 40`, `7 - 9 → -2`.
- Bitwise / shift folding (CLOC15.D): `0xFF & 0x3C → 60`, `1 << 4 | 2 → 18`,
  `8 >>> 1 → 4` — ES `ToInt32`/`ToUint32` 32-bit semantics (`>>>` is unsigned).
- String concatenation: `"foo" + "bar" → "foobar"`.
- Boolean short-circuit: `true && x → x`, `false || y → y`.
- `typeof` of literals: `typeof "s" → "string"`, `typeof 0 → "number"`.
- Negation folding: `!true → false`.
- Comparison: `1 < 2 → true`.
- Conditional folding when condition is constant: `true ? a : b → a`.

Type-aware folding (via the sidecar) lets us avoid folding NaN-vs-NaN
comparisons or anything where the typechecker is uncertain.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — the `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` input/output.
- `coding-adventures-type-sidecar` — type-aware fold safety.
- `coding_adventures_correlation_vector` — `Contribution` plumbing
  per CLOC03.
- `serde_json` — `Contribution.meta` JSON values.

Plus `coding-adventures-javascript-tokens` as a dev-dependency for
`EsVersion` in tests.

## Upstream conformance tests

`tests/upstream/` ports Google Closure Compiler tests (Apache-2.0; see
`ATTRIBUTION.md` and `UPSTREAM_SHA`), per the CLOC12 test-port convention:

- `peephole_fold_constants_test.rs` — `PeepholeFoldConstantsTest` (binary/unary
  constant folding).
- `peephole_replace_known_methods_test.rs` — `PeepholeReplaceKnownMethodsTest`.
  Pins the String-method folds this pass performs (indexOf, lastIndexOf, case
  conversion, slice, substring, substr, charAt, charCodeAt, repeat, trim,
  includes/startsWith/endsWith), the numeric `Math.abs`/`floor`/`ceil`/`round`
  and `Math.max`/`Math.min` folds, the `Array.prototype.join` fold on an
  array literal of constants (gap-142, `["a","b"].join("-")` → `"a-b"`), and
  the `String#concat` fold with `ToString`-coerced primitive arguments (gap-143,
  `"x".concat(1, 2)` → `"x12"`). **This port is now fully active** — gaps
  141/142/143 are all closed, so there are no remaining `#[ignore]`
  placeholders. Run with
  `cargo test --test upstream_peephole_replace_known_methods` (every case is
  active — nothing is ignored).
