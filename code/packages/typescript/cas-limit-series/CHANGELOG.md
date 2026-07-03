# Changelog

## 0.3.0 - 2026-05-29

### Added

- Track J2: Taylor-series-expansion limit fallback (`trySeriesLimit`)
  porting Python Track J1 (PR #5574). Fires inside `limitAdvanced` after
  L'Hopital (or instead of it if no `differentiate` callback is supplied)
  and resolves transcendental `0/0` limits via a self-contained
  rational-coefficient series ring with bounded order (4 → 6 → 8 → 10 →
  12). Closes the canonical acceptance set:
  `limit((sin(x) - x)/x^3, x, 0) = -1/6`,
  `limit((1 - cos(x))/x^2, x, 0) = 1/2`,
  `limit((exp(x) - 1 - x)/x^2, x, 0) = 1/2`,
  `limit((tan(x) - x)/x^3, x, 0) = 1/3`,
  `limit((log(1 + x) - x)/x^2, x, 0) = -1/2`.
- `limitAdvanced(sin(x)/x, x, 0)` now closes to `1` without an injected
  `differentiate` callback (was previously an unevaluated `Limit(...)`).

## 0.2.2 - 2026-06-06

- Add bounded-over-diverging limit recognition at infinity, closing
  `limit(sin(x)/x, x, inf)` and `limit(cos(x)/(x^2+1), x, minf)` to exact
  `0` instead of returning an unevaluated `Limit(...)`.

## 0.1.0

- Add pure TypeScript parity for Rust `cas-limit-series` direct limits and
  polynomial Taylor expansion.
