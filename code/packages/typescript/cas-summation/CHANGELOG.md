# Changelog

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
