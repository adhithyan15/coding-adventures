# Changelog

## 0.2.0 — 2026-05-20

**Phase 39 — Telescoping sum recognition (Rust port).**

Mirrors Python `cas-summation` 0.2.0 (PR #3706 ✅ merged) and the
in-flight TypeScript port (PR #3720).

`evaluate_sum` now detects structurally telescoping summands of the
form `f = g(k+1) − g(k)` (and the antisymmetric `g(k) − g(k+1)`) and
emits the closed form:

    ∑_{k=lo}^{hi} [g(k+1) − g(k)]  =  g(hi+1) − g(lo)
    ∑_{k=lo}^{hi} [g(k) − g(k+1)]  =  g(lo) − g(hi+1)

Detection is purely structural: substitute `k → k+1` in one half of
the `SUB` shape and compare against the other half after `eval_fn`
normalisation.  No partial-fraction expansion is attempted (the
classic `1/(k(k+1))` form needs an explicit `Apart` step first).
Infinite ranges fall through (a future limit-aware phase will handle
those).

### Added

- **`try_telescoping<E>(f, k, eval_fn)`** in `src/lib.rs` — generic
  over `E: FnMut(IRNode) -> IRNode`.  Returns `Some((g_expr, sign))`
  where `sign = 1` for the standard `g(k+1) − g(k)` orientation and
  `-1` for the antisymmetric `g(k) − g(k+1)`.
- New dispatch step inserted between Faulhaber and classic-infinite
  in `evaluate_sum`, guarded on `!inf_upper`.

### Added — tests

`tests/tests.rs` — 8 new `#[test]` functions covering:

- `phase39_standard_telescope_concrete_bounds`: `(k+1)² − k²` → 24.
- `phase39_antisymmetric_telescope`: `k² − (k+1)²` → −15.
- `phase39_linear_g_counts_terms`: `(k+1) − k`.
- `phase39_constant_offset_in_g`: `(k+6) − (k+5)`.
- `phase39_non_telescoping_falls_through`: `k² − k` (numeric path).
- `phase39_constant_difference_routes_through_constant_rule`.
- `phase39_symbolic_upper_bound_non_unevaluated`.
- `phase39_infinite_upper_falls_through`.

All 14 tests pass (6 prior + 8 net new).

## 0.1.0

- Add symbolic summation and product evaluator for Rust.
- Add geometric, Faulhaber, special infinite-series, and product tests.
