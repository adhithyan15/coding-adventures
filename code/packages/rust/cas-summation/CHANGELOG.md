# Changelog

## 0.5.0 — 2026-05-22

**Phase 44 — Log divergence recogniser (Rust port).**

Ports Python `cas-summation` 0.6.0 (PR #3909).  Extends Phase 43's
`h_diverges_at_infinity` to also accept `Log(h(k))` where `h(k) → +∞`.

### Added

- New **Log branch** in `h_diverges_at_infinity` with three sub-cases:
  1. Polynomial inner: positive leading coefficient required.
  2. `Exp(h')` inner: always positive; defer.
  3. `Pow(b, h')` inner: require base `b > 1` *strictly positive*.

### Added — tests

4 new `#[test]` functions:
- `phase44_log_of_polynomial_recognised`.
- `phase44_log_of_exp_recognised`.
- `phase44_log_of_pow_negative_base_refuses` (regression).
- `phase44_log_of_negative_polynomial_refuses` (regression).

Full suite: **31 passed** (27 prior + 4 net new).

## 0.4.0 — 2026-05-22

**Phase 43 — Transcendental vanishing-at-infinity (Rust port).**

Ports Python `cas-summation` 0.5.0 (PR #3899 in review).  Extends the
Phase 41/42 denominator recogniser to accept exponentially diverging
shapes so `∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1` and similar close.

### Added

- **`h_diverges_at_infinity(node, k)`** — union of Phase 41/42
  positive-degree polynomial check and three transcendental cases:
  `Exp(h)`, `Pow(b, h)` with rational `|b| > 1`, and `Mul` of such
  factors.  Each transcendental case requires the polynomial argument
  ``h`` to have a positive leading coefficient (so it really diverges
  to ``+∞``, not ``−∞``).
- **`polynomial_leading_coeff_sign_in_k(node, k) -> Option<i64>`** —
  returns `Some(1)` / `Some(-1)` for the polynomial's leading
  coefficient sign in `k`, `None` for non-polynomial / degree-0 /
  unknown-sign shapes.  Required to refuse `2^(-k)` (it vanishes).

### Changed

- `g_vanishes_at_infinity` Phase 41 fast path now calls
  `h_diverges_at_infinity` instead of
  `is_positive_degree_polynomial_in_k` directly.

### Added — tests

`tests/tests.rs` — 6 new `#[test]` functions:

- `phase43_pow_2_diverges_closes` (= 1).
- `phase43_pow_3_higher_start` (= 1/3).
- `phase43_base_half_falls_through`.
- `phase43_mul_polynomial_times_exponential` (= 1/2).
- `phase43_pow_negative_exponent_polynomial_refuses` (regression).
- `phase43_pow_neg_wrapper_refuses` (regression, NEG wrapper).

Full suite: **27 passed** (21 prior + 6 net new).

### Still deferred

- Apart-induced telescopes — blocked on porting `Apart` to Rust.
- Transcendental limit-finder for non-polynomial shapes.

## 0.3.0 — 2026-05-22

**Phase 41 + Phase 42 — Limit-aware infinite telescope (Rust port).**

Ports Python `cas-summation` 0.3.0 (PR #3880 ✅) and 0.4.0
(PR #3887 ✅) in one go.  Extends `evaluate_sum`'s telescope detection
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

- **`is_positive_degree_polynomial_in_k(node, k)`** — recogniser for
  `k`, `k^n` (n ≥ 1), `Add`, and `Mul` of these.
- **`polynomial_degree_in_k(node, k) -> Option<i64>`** — returns the
  polynomial degree of an IR node in `k`, or `None` for non-polynomial
  shapes (Div, Sin, fractional Pow, …).
- **`g_vanishes_at_infinity(g, k)`** — two-tier predicate combining
  the above.

### Changed

- The `!inf_upper` gate around the Phase 39 telescope branch is
  lifted; the dispatcher now runs telescope detection for both finite
  and infinite ranges and routes through the new vanishing-at-infinity
  check when `hi = %inf`.
- Existing `phase39_infinite_upper_falls_through` test docstring
  updated to reflect that it now pins the Phase 41 guard against
  divergent telescopes (`g(k) = k`).

### Added — tests

`tests/tests.rs` — 7 new `#[test]` functions:

- `phase41_antisymmetric_one_over_k_minus_one_over_kp1` (= 1).
- `phase41_standard_orientation_kp1_minus_k` (= −1).
- `phase41_higher_starting_index` (= 1/2).
- `phase41_quadratic_denominator` (= 1).
- `phase42_proper_rational_k_over_k_squared_plus_1` (= 1/2).
- `phase42_improper_rational_falls_through` (`k/(k+1)`).
- `phase42_transcendental_numerator_falls_through` (`sin(k)/k²`).

Full suite: **21 passed** (14 prior + 7 net new).

### Still deferred

- Apart-induced telescopes (`1/(k(k+1))`) — blocked on porting the
  `Apart` partial-fraction-decomposition handler to Rust.
- Transcendental limit-finder (`sin(k)/k²`, `log(k)/k`, `1/exp(k)`).

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
