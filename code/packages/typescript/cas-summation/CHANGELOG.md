# Changelog

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
