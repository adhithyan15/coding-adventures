# Changelog

## 0.5.0 — 2026-05-22

**Phase 44 — Log divergence recogniser (TypeScript port).**

Ports Python `cas-summation` 0.6.0 (PR #3909).  Extends Phase 43's
`hDivergesAtInfinity` to also accept `Log(h(k))` where `h(k) → +∞`.

### Added

- New **Log branch** in `hDivergesAtInfinity` with three sub-cases:
  1. Polynomial inner: positive leading coefficient required.
  2. `Exp(h')` inner: always positive; defer.
  3. `Pow(b, h')` inner: require base `b > 1` *strictly positive*
     (not just `|b| > 1`; `Pow(-2, k)` value oscillates so
     `log((-2)^k)` not real-valued).

### Added — tests

4 new cases:
- `Log(k+1)` recognised.
- `Log(2^k)` recognised via Phase 43 Pow delegation.
- Regression: `Log(Pow(-2, k))` refused.
- Regression: `Log(Mul(-1, k))` refused.

Full suite: **31 passed** (27 prior + 4 net new).

## 0.4.0 — 2026-05-22

**Phase 43 — Transcendental vanishing-at-infinity (TypeScript port).**

Ports Python `cas-summation` 0.5.0 (PR #3899 in review).  Extends the
Phase 41/42 denominator recogniser to accept exponentially diverging
shapes so `∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1` and similar close.

### Added

- **`hDivergesAtInfinity(node, k)`** — union of Phase 41/42
  positive-degree polynomial check and three transcendental cases:
  1. `Exp(h(k))` with h positive-degree AND positive leading coeff.
  2. `Pow(b, h(k))` with rational `|b| > 1` AND h positive-degree
     with positive leading coefficient.
  3. `Mul(...)` where at least one factor diverges and the others
     are constant-in-k or also diverging.  Recursive.
- **`polynomialLeadingCoeffSignInK(node, k) -> 1 | -1 | undefined`**
  — returns the sign of the polynomial's leading coefficient in `k`,
  or `undefined` for non-polynomial / degree-0 / unknown-sign shapes.
  Required to refuse `exp(-k)`, `2^(-k)`, etc. (these vanish, not
  diverge).

### Changed

- `gVanishesAtInfinity` Phase 41 fast path now calls
  `hDivergesAtInfinity` instead of `isPositiveDegreePolynomialInK`
  directly, picking up the transcendental cases automatically.

### Added — tests

`tests/cas-summation.test.ts` — new
`summation: Phase 43 transcendental vanishing-at-infinity` describe
block with 6 cases:

- `∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1`.
- `∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = 1/3`.
- Base 1/2 falls through (`(1/2)^k → 0`, not ∞).
- `Mul` of polynomial × exponential `k · 2^k` closes (= 1/2).
- Regression: `2^(-k)` via `Mul(-1, k)` does NOT diverge → refuse.
- Regression: `2^(Neg(k))` does NOT diverge → refuse (NEG wrapper).

Full suite: **27 passed** (21 prior + 6 net new).

### Still deferred

- Apart-induced telescopes (e.g. `1/(k(k+1))`) — blocked on porting
  the `Apart` partial-fraction-decomposition handler to TypeScript.
- Transcendental limit-finder for shapes the polynomial-degree path
  doesn't cover (e.g. `sin(k)/k²`, `log(k)/k`).

## 0.3.0 — 2026-05-22

**Phase 41 + Phase 42 — Limit-aware infinite telescope (TypeScript port).**

Ports Python `cas-summation` 0.3.0 (PR #3880 ✅) and 0.4.0
(PR #3887 ✅) in one go.  Extends `evaluateSum`'s telescope detection
to handle `hi = %inf` when `g(k)` provably vanishes at infinity:

    ∑_{k=lo}^∞ [g(k+1) − g(k)]  =  −g(lo)   (standard orientation)
    ∑_{k=lo}^∞ [g(k) − g(k+1)]  =   g(lo)   (antisymmetric)

The vanishing-at-infinity check uses two tiers:

1.  **Phase 41 fast path** — `Div(constant-in-k, h(k))` with `h` a
    positive-degree polynomial in `k`.
2.  **Phase 42 widening** — `Div(P(k), Q(k))` where both are pure
    polynomials and `deg(P) < deg(Q)`.

Anything transcendental, improper, or non-Div falls through to the
unevaluated `Sum(...)`.

### Added

- **`isPositiveDegreePolynomialInK(node, k)`** — recogniser for ``k``,
  ``k^n`` (n ≥ 1), ``Add``, and ``Mul`` of these.
- **`polynomialDegreeInK(node, k) -> number | undefined`** — returns
  the polynomial degree of an IR node in ``k``, or undefined for
  non-polynomial shapes (Div, Sin, fractional Pow, …).
- **`gVanishesAtInfinity(g, k)`** — two-tier predicate combining the
  above.

### Changed

- The `infUpper` gate around the Phase 39 telescope branch is lifted;
  the dispatcher now runs telescope detection for both finite and
  infinite ranges and routes through the new vanishing-at-infinity
  check when `hi = %inf`.
- Existing "infinite upper bound falls through" test renamed to
  pin the Phase 41 guard against divergent telescopes
  (`g(k) = k` doesn't vanish).

### Added — tests

`tests/cas-summation.test.ts` — new
`summation: Phase 41+42 limit-aware infinite telescope` describe
block with 7 cases:

- `∑_{k=1}^∞ [1/k − 1/(k+1)] = 1` (Phase 41 antisymmetric).
- `∑_{k=1}^∞ [1/(k+1) − 1/k] = −1` (standard orientation).
- Higher starting index `∑_{k=2}^∞ … = 1/2`.
- Quadratic denominator `∑ 1/k² − 1/(k+1)² = 1`.
- Phase 42 proper rational
  `∑ k/(k²+1) − (k+1)/((k+1)²+1) = 1/2`.
- Improper rational `k/(k+1)` falls through (limit is 1).
- Transcendental `sin(k)/k²` falls through (non-polynomial).

Full suite: **21 passed** (14 prior + 7 net new).

### Still deferred

- Apart-induced telescopes (`1/(k(k+1))`) — blocked on porting the
  `Apart` partial-fraction-decomposition handler to TypeScript.
- Transcendental limit-finder (`sin(k)/k²`, `log(k)/k`, `1/exp(k)`).

## 0.2.0 — 2026-05-20

**Phase 39 — Telescoping sum recognition (TypeScript port).**

Mirrors Python `cas-summation` 0.2.0 (PR #3706 ✅ merged).

The `evaluateSum` dispatcher now detects structurally telescoping
summands of the form `f = g(k+1) − g(k)` (and the antisymmetric
`g(k) − g(k+1)`) and emits the closed form:

    ∑_{k=lo}^{hi} [g(k+1) − g(k)]  =  g(hi+1) − g(lo)
    ∑_{k=lo}^{hi} [g(k) − g(k+1)]  =  g(lo) − g(hi+1)

Detection is purely structural: substitute `k → k+1` in one half of
the `SUB` shape and compare against the other half after `evalFn`
normalisation.  No partial-fraction expansion is attempted — the
classic `1/(k(k+1))` example becomes telescoping only after an
explicit `Apart` step, left for a follow-on phase.  Infinite ranges
fall through (a future limit-aware phase will handle those).

### Added

- **`tryTelescoping(f, k, evalFn)`** in `src/index.ts` — returns
  `{ gExpr, sign }` when the SUB structure matches, where `sign = 1`
  for the standard `g(k+1) − g(k)` orientation and `-1` for the
  antisymmetric `g(k) − g(k+1)`.
- New dispatch step inserted between Faulhaber and classic-infinite,
  guarded on `!infUpper`.

### Added — tests

`tests/cas-summation.test.ts` — new `summation: Phase 39 telescoping`
describe block with 8 cases covering:

- Standard `(k+1)² − k²` telescope at concrete bounds → 24.
- Antisymmetric `k² − (k+1)²` orientation → −15.
- Linear `g(k) = k` (`f ≡ 1` counts terms).
- `g(k) = k + 5` (constant offset preserved through substitution).
- Non-telescoping `k² − k` falls through to numeric/Faulhaber.
- Constant-difference summand routes through step 1 (constant rule).
- Symbolic upper bound `n` produces a non-unevaluated tree.
- Infinite upper bound correctly stays unevaluated.

All 14 tests pass (6 prior + 8 net new).

## 0.1.0

- Add pure TypeScript summation and product evaluator.
- Cover geometric, Faulhaber, classic infinite-series, and product patterns.
