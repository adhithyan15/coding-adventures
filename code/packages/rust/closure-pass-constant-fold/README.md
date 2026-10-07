# coding-adventures-closure-pass-constant-fold

The **first concrete optimization pass** for the Closure Compiler
clone. Folds compile-time-evaluable expressions per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s canonical
pass set.

## Implemented behavior

The pass performs a bottom-up walk. It folds arithmetic, bitwise operations, primitive comparisons, string
members/methods and other guarded constant operations. See the changelog and
ported upstream tests for the implemented families.

[CLOC31](../../../specs/CLOC31-primitive-fold-lineage.md) repairs traced
string-literal `.length`, primitive binary and primitive unary folds: each
replacement retains available operand CVs and owns its `constant-fold/folded`
contribution. Nested folds retain child history. Other families still require
the CCR-065 lineage audit; this is not a claim of complete output-byte tracing.

Equal conditional branches use borrowed AST comparison excluding CV metadata,
including values nested inside composite branches. Signed zero and every other
represented field remain distinct. Primitive branch collapse retains both
histories; composite branch lineage remains in CCR-065.

- `ConstantFoldPass` implementing the `Pass` trait from
  [`coding-adventures-closure-pass-pipeline`](../closure-pass-pipeline).
- `name = "constant-fold"`, `iteration_policy = FixedPoint`, `cost = 2`.
- The bottom-up walk handles nested folds in one pass; later pipeline passes
  may expose new foldable expressions for the next sweep.
- A large-stack worker protects the recursive walk on deeply nested inputs.

Examples of implemented folds:

- Number folding: `2 + 3 → 5`, `10 * 4 → 40`, `7 - 9 → -2`.
- Bitwise / shift folding (CLOC15.D): `0xFF & 0x3C → 60`, `1 << 4 | 2 → 18`,
  `8 >>> 1 → 4` — ES `ToInt32`/`ToUint32` 32-bit semantics (`>>>` is unsigned).
- String concatenation: `"foo" + "bar" → "foobar"`.
- Boolean short-circuit: `true && x → x`, `false || y → y`.
- `typeof` of literals: `typeof "s" → "string"`, `typeof 0 → "number"`.
- Negation folding: `!true → false`.
- Comparison: `1 < 2 → true`.
- Conditional folding when condition is constant: `true ? a : b → a`.

Each family has guards for supported input shapes, coercion and observable
effects. Division by zero is deliberately retained rather than replaced with
the shadowable `Infinity` or `NaN` names. The typechecker remains a separate
passthrough scaffold; this pass does not claim inference-backed safety.

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

## Checked provenance failures (CV02)

The recursive visitor retains its first derivation, merge or contribution
error. `Pass::run` returns it before accepting the transformed program; later
forks stop recording. Checked resource exhaustion is an error, not a panic or
an accepted replacement with missing evidence.

On a provenance error the worker disposes of the candidate AST on its own
large stack before returning, so rejection does not transfer a deep tree to
a small caller stack for destruction.
