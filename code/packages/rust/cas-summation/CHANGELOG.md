# Changelog

## 2.37.0 — 2026-05-27

### Added

- **Phase 100 — Five-Sqrt × Eight-Log × polynomial numerator** (`five_sqrt_eight_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), ..., Sqrt(P5), Log(h1), ..., Log(h8), polynomial..., bounded...)`.
  Exactly 5 Sqrt factors and exactly 8 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  Completes the Eight-Log family (Phases 95–100).
  - 3 new tests (`phase100_*`).

## 2.36.0 — 2026-05-27

### Added

- **Phase 99 — Four-Sqrt × Eight-Log × polynomial numerator** (`four_sqrt_eight_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1), ..., Log(h8), polynomial..., bounded...)`.
  Exactly 4 Sqrt factors and exactly 8 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new tests (`phase99_*`).

## 2.35.0 — 2026-05-27

### Added

- **Phase 98 — Three-Sqrt × Eight-Log × polynomial numerator** (`three_sqrt_eight_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), ..., Log(h8), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 8 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new tests (`phase98_*`).

## 2.34.0 — 2026-05-27

### Added

- **Phase 97 — Two-Sqrt × Eight-Log × polynomial numerator** (`two_sqrt_eight_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1), ..., Log(h8), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 8 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  - 3 new tests.

## 2.33.0 — 2026-05-27

### Added

- **Phase 96 — One-Sqrt × Eight-Log × polynomial numerator** (`one_sqrt_eight_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1), ..., Log(h8), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 8 Log factors required.
  `effective_x2 = sqrt_deg_x2 + 2·poly_deg`.
  - 3 new tests: `phase96_sqrt_k_log8_k_over_k2_closes`,
    `phase96_sqrt_k2_log8_k_over_k3_closes`,
    `phase96_sqrt_k_log8_k_times_k_over_k_refused`.

## 2.32.0 — 2026-05-27

### Added

- **Phase 95 — Eight-Log × polynomial numerator** (`eight_log_poly_effective_x2`):
  recognises `Mul(Log(h1), ..., Log(h8), polynomial..., bounded...)` with exactly 8 Log
  factors and no Sqrt factors.  `log⁸(k)` is sub-polynomial (`o(k^ε)`), contributing 0;
  `effective_x2 = 2·poly_deg`.  Closes when `2·den_deg > effective_x2` or non-polynomial
  diverging denominator.  Opens the Eight-Log family (Phases 95–100).
  - 3 new tests: `phase95_log8_k_over_k2_closes`, `phase95_log8_k_times_k_over_k3_closes`,
    `phase95_log8_k_times_k_over_k_refused`.

## 2.31.0 — 2026-05-27

### Added

- **Phase 94 — Five-Sqrt × Seven-Log × polynomial numerator** (`five_sqrt_seven_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1)..Sqrt(P5), Log(h1), ..., Log(h7), polynomial..., bounded...)`.
  Exactly 5 Sqrt factors and exactly 7 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  Completes the Seven-Log family (Phases 89–94).
  - 3 new tests: `phase94_sqrt_k_x5_log7_k_over_k4_closes`,
    `phase94_sqrt_k2_x5_log7_k_over_k6_closes`,
    `phase94_sqrt_k_x5_log7_k_times_k_over_k3_refused`.

## 2.30.0 — 2026-05-27

### Added

- **Phase 93 — Four-Sqrt × Seven-Log × polynomial numerator** (`four_sqrt_seven_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1)..Sqrt(P4), Log(h1), ..., Log(h7), polynomial..., bounded...)`.
  Exactly 4 Sqrt factors and exactly 7 Log factors required.
  `effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg`.
  - 3 new tests: `phase93_sqrt_k_x4_log7_k_over_k3_closes`,
    `phase93_sqrt_k2_x4_log7_k_over_k5_closes`,
    `phase93_sqrt_k_x4_log7_k_times_k_over_k3_refused`.

## 2.29.0 — 2026-05-27

### Added

- **Phase 92 — Three-Sqrt × Seven-Log × polynomial numerator** (`three_sqrt_seven_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1(k)), Sqrt(P2(k)), Sqrt(P3(k)), Log(h1(k)), ..., Log(h7(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 7 Log factors required.  `log⁷(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new tests: `phase92_sqrt_k_sqrt_k_sqrt_k_log7_k_over_k2_closes`,
    `phase92_sqrt_k3_sqrt_k3_sqrt_k3_log7_k_over_k5_closes`,
    `phase92_sqrt_k_sqrt_k_sqrt_k_log7_k_times_k_over_k2_refused`.

## 2.28.0 — 2026-05-27

### Added

- **Phase 91 — Two-Sqrt × Seven-Log × polynomial numerator** (`two_sqrt_seven_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1(k)), Sqrt(P2(k)), Log(h1(k)), ..., Log(h7(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 7 Log factors required.  `log⁷(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`.
  Closes when `2·den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new tests: `phase91_sqrt_k_sqrt_k_log7_k_over_k2_closes`,
    `phase91_sqrt_k3_sqrt_k3_log7_k_over_k4_closes`,
    `phase91_sqrt_k_sqrt_k_log7_k_times_k_over_k2_refused`.

## 2.27.0 — 2026-05-26

### Added

- **Phase 89 — Seven-Log × polynomial numerator** (`seven_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), ..., Log(h7(k)), polynomial..., bounded...)`.
  Exactly 7 Log factors required; Sqrt factors explicitly refused.  `log⁷(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase89_*`).
- **Phase 90 — One-Sqrt × Seven-Log × polynomial numerator** (`one_sqrt_seven_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), ..., Log(h7(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt and exactly 7 Log factors required.  `log⁷(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase90_*`).

## 2.21.0 — 2026-05-25

### Added

- **Phase 81 — Four-Sqrt × Five-Log × polynomial numerator** (`four_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 4 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase81_*`).

## 2.20.0 — 2026-05-25

### Added

- **Phase 80 — Three-Sqrt × Five-Log × polynomial numerator** (`three_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), ..., Log(h5(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase80_*`).

## 2.19.0 — 2026-05-25

### Added

- **Phase 84 — One-Sqrt × Six-Log × polynomial numerator** (`one_sqrt_six_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 6 Log factors required.  `log⁶(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase84_*`).

## 2.18.0 — 2026-05-25

### Added

- **Phase 82 — Five-Sqrt × Five-Log × polynomial numerator** (`five_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Sqrt(P5), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase82_*`).
- **Phase 83 — Six-Log × polynomial numerator** (`six_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), Log(h6(k)), polynomial..., bounded...)`.
  Exactly 6 Log factors and zero Sqrt factors required.  `log⁶(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase83_*`).

## 2.17.0 — 2026-05-25

### Added

- **Phase 79 — Two-Sqrt × Five-Log × polynomial numerator** (`two_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase79_*`).

## 2.16.0 — 2026-05-25

### Added

- **Phase 78 — One-Sqrt × Five-Log × polynomial numerator** (`one_sqrt_five_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 5 Log factors required.  `log⁵(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt_inner_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase78_*`).

## 2.15.0 — 2026-05-25

### Added

- **Phase 77 — Five-Log × polynomial numerator** (`five_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), Log(h5(k)), polynomial..., bounded...)`.
  Exactly 5 Log factors; Sqrt factors explicitly refused so Sqrt-bearing phases (73–76, 78+) are
  not shadowed.  `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase77_*`).

## 2.14.0 — 2026-05-25

### Added

- **Phase 76 — Three-Sqrt × Four-Log × polynomial numerator** (`three_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase76_*`).

## 2.13.0 — 2026-05-25

### Added

- **Phase 75 — Two-Sqrt × Four-Log × polynomial numerator** (`two_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree;
  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase75_*`).

## 2.12.0 — 2026-05-25

### Added

- **Phase 74 — One-Sqrt × Four-Log × polynomial numerator** (`one_sqrt_four_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 4 Log factors required.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = sqrt_inner_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase74_*`).

## 2.11.0 — 2026-05-25

### Added

- **Phase 73 — Four-Log × polynomial numerator** (`four_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), Log(h4(k)), polynomial..., bounded...)`.
  Exactly 4 Log factors required; Sqrt factors are refused.  `log⁴(k)` is sub-polynomial
  (`o(k^ε)`), contributing 0 to effective degree; `effective_x2 = 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs` (`phase73_*`).

## 2.10.0 — 2026-05-25

### Added

- **Phase 72 — Three-Sqrt × Three-Log × polynomial numerator** (`three_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.9.0 — 2026-05-25

### Added

- **Phase 71 — Two-Sqrt × Three-Log × polynomial numerator** (`two_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 2 Sqrt factors and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.8.0 — 2026-05-25

### Added

- **Phase 70 — Three-Sqrt × Two-Log × polynomial numerator** (`three_sqrt_two_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 2 Log factors required; log² sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.7.0 — 2026-05-25

### Added

- **Phase 69 — One-Sqrt × Three-Log × polynomial numerator** (`one_sqrt_three_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P), Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Exactly 1 Sqrt factor and exactly 3 Log factors required; log³ sub-polynomial contributes 0
  to effective degree; `effective_x2 = sqrt_deg + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.6.0 — 2026-05-25

### Added

- **Phase 68 — Three-Sqrt × Log × polynomial numerator** (`three_sqrt_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h(k)), polynomial..., bounded...)`.
  Exactly 3 Sqrt factors and exactly 1 Log factor required; log sub-polynomial contributes 0
  to effective degree; `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.5.0 — 2026-05-25

### Added

- **Phase 67 — Three-Log × polynomial numerator** (`three_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Log(h3(k)), polynomial..., bounded...)`.
  Sqrt factors refused; log³ sub-polynomial; `effective_x2 = 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.4.0 — 2026-05-25

### Added

- **Phase 66 — Three-Sqrt × polynomial numerator** (`three_sqrt_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...)`.
  Log factors refused (use Phase 63/64/65 for sqrt+log combos);
  `effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.3.0 — 2026-05-25

### Added

- **Phase 65 — Two-Sqrt × Two-Log × polynomial numerator** (`two_sqrt_two_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h1(k)), Log(h2(k)), polynomial..., bounded...)`.
  log² sub-polynomial; `effective_x2 = deg(P1) + deg(P2) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.2.0 — 2026-05-25

### Added

- **Phase 64 — Two-Log × Sqrt × polynomial numerator** (`two_log_sqrt_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), Sqrt(P), polynomial..., bounded...)`.
  log² sub-polynomial; `effective_x2 = sqrt_deg_x2 + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.1.0 — 2026-05-25

### Added

- **Phase 63 — Two-Sqrt × Log × polynomial numerator** (`two_sqrt_log_poly_effective_x2`):
  recognises `Mul(Sqrt(P1), Sqrt(P2), Log(h(k)), polynomial..., bounded...)`.
  Log is sub-polynomial; `effective_x2 = deg(P1) + deg(P2) + 2 * poly_deg`.
  Closes when `2 * den_deg > effective_x2` or non-polynomial diverging denominator.
  - 3 new integration tests in `tests/tests.rs`.

## 2.0.0 — 2026-05-25

### Added

- **Phase 62 — Two-Log × polynomial numerator** (`two_log_poly_effective_x2`):
  recognises `Mul(Log(h1(k)), Log(h2(k)), polynomial..., bounded...)` as a
  numerator that vanishes at infinity.  `effective_x2 = 2 * poly_deg` (log²
  grows sub-polynomially).  Closes when `2 * den_deg > effective_x2` or
  denominator is non-polynomial diverging.  Sqrt factors refused.
  - 3 new integration tests in `tests/tests.rs`.

## 1.9.0 — 2026-05-25

**Phase 61 — Two-Sqrt × polynomial numerator (Rust port).**

Ports Python ``cas-summation`` 1.9.0.  Closes the gap where all prior Sqrt
phases (51, 53, 56, 59, 60) hard-reject a second Sqrt operand.

Effective growth: ``k^{deg(P1)/2 + deg(P2)/2 + m}``.
×2 trick: ``effective_x2 = deg(P1) + deg(P2) + 2·m``.
Vanishes when ``2·den_deg > effective_x2`` or non-polynomial diverging denom.

### Added

- **`two_sqrt_poly_effective_x2(node, k) -> Option<i64>`** — returns
  ``deg(P1) + deg(P2) + 2·poly_deg`` for
  ``Mul(Sqrt(P1), Sqrt(P2), poly_factors..., bounded_factors...)``.
  Refuses three-or-more Sqrt, any Log factor, or unrecognised factors.
- **Phase 61 branch** in ``g_vanishes_at_infinity`` — checks
  ``2·den_deg > tsp_x2`` for polynomial denominators; falls back to
  ``h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **3 new tests**: ``phase61_*`` in ``tests/tests.rs``.

## 1.8.0 — 2026-05-24

**Phase 60 — Bounded × Log(diverging) × Sqrt(positive-poly) × polynomial
numerator (Rust port).**

Ports Python ``cas-summation`` 1.8.0.  Closes the gap left by Phase 57
(``Mul(bounded, Log, Sqrt)``; refuses polynomial factors).

Effective growth: ``log(k)·k^{deg(P)/2 + m}`` — log is sub-polynomial so
the dominant term is the Sqrt×poly part.  ×2 trick:
``effective_x2 = deg(P) + 2·m``.  Vanishes when
``2·den_deg > effective_x2`` (polynomial denominator) or non-polynomial
diverging denominator.

### Added

- **`bounded_log_sqrt_poly_effective_x2(node, k) -> Option<i64>`** — returns
  ``sqrt_inner_deg + 2·poly_deg`` for
  ``Mul(bounded..., Log(diverging), Sqrt(positive-poly), poly_factors...)``.
  Requires exactly one Log and exactly one Sqrt; refuses two of either.
- **Phase 60 branch** in ``g_vanishes_at_infinity`` — checks
  ``2·den_deg > blsp_x2`` for polynomial denominators; falls back to
  ``h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **3 new tests**: ``phase60_*`` in ``tests/tests.rs``.

## 1.7.0 — 2026-05-25

**Phase 59 — Bounded × Sqrt(positive-poly) × polynomial numerator (Rust port).**

Ports Python ``cas-summation`` 1.7.0.  Fills the gap between Phase 53
(Sqrt × polynomial, refuses bounded factors) and Phase 56 (bounded × Sqrt,
refuses polynomial factors).

Effective growth: ``C·k^{deg(P)/2 + poly_deg}``.  ×2 trick:
``effective_x2 = deg(P) + 2·poly_deg``.  Vanishes when
``2·den_deg > effective_x2`` or non-polynomial diverging denominator.

### Added

- **`bounded_sqrt_poly_effective_x2(node, k) -> Option<i64>`** — returns
  ``sqrt_inner_deg + 2·poly_deg`` for
  ``Mul(bounded..., Sqrt(positive-poly), poly_factors...)``.
  Refuses two-Sqrt, Log (→ Phase 57), or unrecognised factors.
- **Phase 59 branch** in ``g_vanishes_at_infinity`` — checks
  ``2·den_deg > bsp_x2`` for polynomial denominators; falls back to
  ``h_diverges_at_infinity`` for non-polynomial diverging denominators.
- **3 new tests**: ``phase59_*`` in ``tests/tests.rs``.

## 1.6.0 — 2026-05-25

**Phase 58 — Bounded × Log(diverging) × polynomial numerator (Rust port).**

Ports Python ``cas-summation`` 1.6.0.  Fills the gap between Phase 54
(Log × polynomial, refuses bounded) and Phase 55 (bounded × Log, refuses
polynomial).

### Added

- **`bounded_log_poly_degree(node, k) -> Option<i64>`** — returns total
  polynomial degree for ``Mul(bounded..., Log(diverging), poly_factors...)``.
  Refuses two-Log, Sqrt (→ Phase 57), or unrecognised factors.

### Changed

- ``g_vanishes_at_infinity`` adds Phase 58 branch after Phase 57:
  ``den_deg > poly_deg`` for polynomial denominators, or
  ``h_diverges_at_infinity`` for non-polynomial diverging denominators.

### Tests

3 new ``phase58_*`` cases.  Full suite: **75 passed** (was 72; +3).

## 1.5.0 — 2026-05-24

**Phase 57 — Bounded × Log(diverging) × Sqrt(positive-poly) numerator
(Rust port).**

Ports Python ``cas-summation`` 1.5.0 (PR #4215).  Closes the mixed
sub-polynomial gap left by Phase 55 (bounded × Log) and Phase 56
(bounded × Sqrt).

### Added

- **`bounded_log_sqrt_inner_deg(node, k) -> Option<i64>`** — returns
  ``deg(P)`` (×2 half-degree to stay in i64 arithmetic) for ``Mul``
  with exactly one ``Log(diverging)`` AND one ``Sqrt(positive-poly)``
  factor (plus optional bounded factors).  Returns ``None`` for zero/
  two-Log, zero/two-Sqrt, or unrecognised factors.

### Changed

- ``g_vanishes_at_infinity`` adds Phase 57 branch after Phase 56,
  comparing ``2 * den_deg > deg(P)`` for polynomial denominators or
  short-circuiting on non-polynomial divergence.

### Tests

3 new ``phase57_*`` cases.  Full suite: **72 passed** (was 69; +3).

## 1.4.0 — 2026-05-23

**Phase 56 — Bounded × Sqrt(diverging) numerator pattern (Rust port).**

Ports Python ``cas-summation`` 1.4.0 (PR #4167).  Bounded × sqrt
analogue of Phase 55's bounded × log.

### Added

- **`bounded_times_sqrt_inner_deg(node, k) -> Option<i64>`** —
  returns ``deg(P)`` (×2 half-degree to stay in i64 arithmetic) for
  ``Mul`` of exactly one ``Sqrt(positive-poly)`` factor and rest
  bounded.  Returns ``None`` for the no-Sqrt case, two-Sqrt case
  (conservative), or unrecognised factors.

### Changed

- ``g_vanishes_at_infinity`` adds Phase 56 branch after Phase 55,
  comparing ``2 * den_deg > deg(P)`` for polynomial denominators or
  short-circuiting on non-polynomial divergence.

### Tests

3 new ``phase56_*`` cases.  Full suite: **69 passed** (was 66; +3).

## 1.3.0 — 2026-05-23

**Phase 55 — Bounded×Log(diverging) numerator pattern (Rust port).**

Ports Python `cas-summation` 1.3.0 Phase 55 to Rust.  Adds
`is_bounded_times_log_in_k` helper and a Phase 55 branch in
`g_vanishes_at_infinity`.  `bounded(k) × log(h(k))` grows sub-polynomially
(log is dominated by any polynomial denominator).

Bumps 1.2.0 → 1.3.0.

### Added

- **`is_bounded_times_log_in_k(node, k)`** — Phase 55 helper. Returns true
  when `node` is a `Mul` with exactly one `Log(diverging)` factor and all
  remaining factors pass `is_bounded_in_k`. Requires exactly one log factor.

- **Phase 55 branch in `g_vanishes_at_infinity`** — after Phase 54, before
  Phase 42. Closes `Div(Mul(bounded, Log(diverging)), den)` when `den`
  diverges (`h_diverges_at_infinity` returns true).

- **5 new tests**:
  - `phase55_sin_k_times_log_k_over_k_squared_closes`
  - `phase55_cos_k_times_log_k_over_k_closes`
  - `phase55_sin_cos_times_log_over_k_cubed_closes`
  - `phase55_sin_times_log_k_squared_over_k_cubed_closes`
  - `phase55_bounded_times_log_constant_denominator_stays` (refused)

Total: 66 tests (was 61).

## 1.2.0 — 2026-05-23

**Phase 54 — Log×polynomial numerator pattern (Rust port).**

Ports Python `cas-summation` 1.2.0 Phase 54 to Rust.  Adds
`split_log_polynomial_factor` helper and a Phase 54 branch in
`g_vanishes_at_infinity`.  Uses the same sub-polynomial growth argument
as the Python/TS ports: `log(h(k)) = o(k^ε)` for any `ε > 0`.

Bumps 1.1.0 → 1.2.0.

### Added

- **`split_log_polynomial_factor<'a>(node, k)`** — Phase 54 helper.
  Splits a `Mul` node into exactly one `Log(diverging)` factor (by ref)
  and a summed polynomial degree; returns `Some((&IRNode, i64))` or
  `None`.

- **Phase 54 branch in `g_vanishes_at_infinity`** — inserted after
  Phase 53 and before Phase 42.  Closes
  `Div(Mul(Log(diverging), P), Q)` when `den_deg > poly_deg`.

- **5 new tests** (`#[test] fn phase54_*`):
  - `phase54_log_k_times_k_over_k_cubed_closes` — poly_deg=1 < 3
  - `phase54_log_k_times_k_squared_over_k_cubed_closes` — poly_deg=2 < 3
  - `phase54_log_k_times_k_over_k_squared_closes` — poly_deg=1 < 2
  - `phase54_log_k_times_k_squared_over_k_squared_stays` — equal degrees
  - `phase54_regression_log_k_over_k_cubed_still_closes_via_phase50`

### Tests

61 passed (was 56; +5 net new — Phase 54).

---

## 1.1.0 — 2026-05-23

**Phase 53 — Sqrt × polynomial numerator pattern (Rust port).**

Extends ``g_vanishes_at_infinity`` to recognise that
``Mul(Sqrt(P), polynomial_factors)`` numerators have effective growth
equal to ``deg(P)/2 + deg(Q)``.  Closes telescopes like
``sqrt(k)·k/k³`` and ``sqrt(k²)·k/k³`` that fall through all
earlier phases.  Uses ×2 integer arithmetic to avoid float comparisons.

Builds on Phase 51 (0.9.0) which added the plain-``Sqrt`` case.
Bumps 1.0.0 → 1.1.0.

### Added

- **``sqrt_poly_numerator_effective_degree_x2(node, k)``** — returns
  ``deg(P) + 2·deg(Q)`` (an ``i64``) when
  ``node = Mul(Sqrt(P), polynomial_factors)`` with exactly one Sqrt
  factor and all others polynomial.  Returns ``None`` for plain
  ``Sqrt`` nodes (handled by Phase 51), non-Mul nodes, multiple Sqrt
  factors, non-polynomial non-Sqrt factors, and negative-leading-coeff
  inner polynomials.

### Changed

- ``g_vanishes_at_infinity`` adds a Phase 53 branch between Phase 52
  (bounded × polynomial) and Phase 42 (pure rational degree comparison):
  closes when ``2 * den_deg > sqrt_poly_numerator_effective_degree_x2(num, k)``.

### Added — tests

5 new ``phase53_*`` cases:
- ``phase53_sqrt_k_times_k_over_k_cubed_closes`` — eff x2 = 3, 2·3 = 6 > 3.
- ``phase53_sqrt_k_squared_times_k_over_k_cubed_closes`` — eff x2 = 4, 6 > 4.
- ``phase53_sqrt_k_times_k_squared_over_k_cubed_closes`` — eff x2 = 5, 6 > 5.
- ``phase53_sqrt_k_times_k_squared_over_k_squared_stays`` — eff x2 = 5, 4 not > 5.
- ``phase53_regression_sqrt_k_over_k_squared_still_closes_via_phase51`` — plain
  Sqrt bypasses Phase 53 and closes via Phase 51.

Full suite: **56 passed** (was 51; +5 net new).

## 1.0.0 — 2026-05-23

**Phase 52 — Bounded × polynomial numerator pattern (Rust port).**

Ports Python ``cas-summation`` 1.0.0.  Extends ``g_vanishes_at_infinity``
to recognise that ``Mul(bounded, polynomial)`` numerators have effective
growth equal to the polynomial part's degree.  Closes telescopes like
``sin(k)·k/k³``, ``k·cos(k)/k²``, where the numerator mixes a bounded
factor with a non-trivial polynomial factor.

Bumps 0.9.0 → 1.0.0.

### Added

- **`split_bounded_polynomial_factor(node, k)`** — partitions a ``Mul``
  node's factors into a bounded aggregate and a summed polynomial degree;
  returns ``None`` if any factor is neither bounded nor polynomial,
  or if no non-constant-in-k bounded factor exists (those go through
  Phase 42).

### Changed

- ``g_vanishes_at_infinity`` now has a Phase 52 branch between Phase 51
  (sqrt numerator) and Phase 42 (degree-aware): when the numerator
  factors as ``bounded × polynomial`` with positive polynomial degree,
  the quotient vanishes iff the denominator's polynomial degree strictly
  exceeds the polynomial part's degree.

### Added — tests

5 new ``phase52_*`` cases:
- ``phase52_sin_k_times_k_over_k_cubed_closes``
- ``phase52_k_times_cos_k_over_k_squared_closes``
- ``phase52_sin_k_times_k_squared_over_k_cubed_closes``
- ``phase52_sin_k_times_k_squared_over_k_squared_stays`` (regression)
- ``phase52_regression_k_over_k_squared_still_closes_via_phase42`` (regression)

Full suite: **51 passed** (was 46; +5 net new).

## 0.9.0 — 2026-05-22

**Phase 51 — Sqrt(polynomial)/polynomial recogniser (Rust port).**

Ports Python ``cas-summation`` 0.9.0.  Recognises that ``sqrt(P(k))``
has effective polynomial degree ``deg(P)/2``; quotient vanishes when
denominator degree exceeds half-degree.

Bumps 0.8.0 → 0.9.0.

### Added

- **`sqrt_effective_half_degree_x2(node, k) -> Option<i64>`** —
  returns ``deg(P)`` (twice the half-degree) for ``Sqrt(P(k))`` with
  positive-leading-coefficient ``P``.  Caller compares with
  ``2 * den_deg`` to preserve the inequality without floats.

### Tests

3 new ``phase51_*`` cases.  Full suite: **46 passed** (was 43; +3).

## 0.8.0 — 2026-05-22

**Phase 50 — Log/polynomial growth-rate recogniser (Rust port).**

Ports Python ``cas-summation`` 0.8.0.  Extends ``g_vanishes_at_infinity``
to accept ``Div(Log(diverging), diverging)`` shapes via the squeeze
argument: ``log(h) → ∞`` at a logarithmic rate, denominator grows
strictly faster, so ``log/poly → 0``.

Builds on Phase 49 (0.7.0) which added ``is_bounded_in_k`` for bounded
× vanishing shapes.

### Added

- **`is_log_of_diverging_in_k(node, k)`** — recognises ``Log(h(k))``
  with ``h(k) → +∞``.  Sign-aware via ``h_diverges_at_infinity``
  (refuses ``Log(Mul(-1, k))``-style shapes).

### Changed

- ``g_vanishes_at_infinity`` adds the Phase 50 branch after the Phase 49
  bounded check and before the Phase 42 degree-aware path.
- The ``phase49_log_numerator_still_refused`` regression is superseded
  and removed — ``log(k)/k²`` now closes via Phase 50.

### Added — tests

3 new ``phase50_*`` cases:
- ``phase50_log_over_k_squared_closes``
- ``phase50_log_of_polynomial_argument_closes``
- ``phase50_log_of_negative_argument_refused`` (regression)

Full suite: **43 passed** (was 41; +2 net new — Phase 49 log regression
superseded by Phase 50 log-closes case).

## 0.7.0 — 2026-05-22

**Phase 49 — Bounded × vanishing recogniser (Rust port).**

Ports Python ``cas-summation`` 0.7.0.  Extends ``g_vanishes_at_infinity``
to accept ``Div(bounded, diverging)`` shapes where the numerator is
uniformly bounded.  Closes telescopes like
``∑ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1)`` that the Phase 42
degree-aware path refused.

### Added

- **`is_bounded_in_k(node, k)`** — recogniser for uniformly bounded
  shapes: constants in ``k``, ``Sin(...)``, ``Cos(...)``, closures
  under ``Mul``/``Add``/``Neg``.  Conservative — returns false for
  anything else.

### Changed

- ``g_vanishes_at_infinity`` now consults ``is_bounded_in_k`` on
  the numerator between the Phase 41 fast-path and the Phase 42
  degree-aware path.

### Added — tests

`tests/tests.rs` — 4 new ``phase49_*`` cases plus the renamed
``phase42_transcendental_numerator_closes_via_phase49`` (assertion
flipped from "stays unevaluated" to "now closes"):

- ``phase49_sin_over_k_squared_closes``
- ``phase49_cos_over_k_cube_closes``
- ``phase49_sin_cos_product_over_diverging``
- ``phase49_log_numerator_still_refused`` (regression)

Full suite: **41 passed** (was 37; +4 net new).

## 0.6.0 — 2026-05-22

**Phase 40+46 — Add-with-negation telescope normaliser (Rust port).**

Ports the Python helpers ``_extract_negation`` and
``_normalise_add_neg_to_sub`` (introduced in ``symbolic-vm``
0.50/0.70).  Widens ``try_telescoping`` to accept summands written in
``Add(g(k+1), Neg(g(k)))`` or ``Add(g(k+1), Div(-c, d))`` form by
rewriting them to the canonical ``Sub`` shape before the structural
match runs.

### Why this is useful in Rust even without ``Apart``

The Python ``Apart`` step (``symbolic-vm`` 0.50/0.70) emits
``Add(Div(-c, k+1), Div(c, k))``, exactly the shape the new
normaliser targets.  On the Rust side ``cas-summation`` doesn't own
an ``Apart`` implementation, but users (or upstream pipelines) who
emit the same shape directly now get the telescope closure for free.

### Added

- **`extract_negation(node: &IRNode) -> Option<IRNode>`** — uniformly
  detects a negation in two recognised forms:
  1.  Top-level ``Neg(x)``                         → ``x``
  2.  ``Div(c, d)`` with literal ``c < 0``         → ``Div(|c|, d)``.
  Handles ``IRNode::Integer``, ``IRNode::Rational``, and
  ``IRNode::Float`` numerators.
- **`normalise_add_neg_to_sub(node: &IRNode) -> IRNode`** — rewrites
  two-term ``Add`` containing a recognised negation into the
  equivalent ``Sub`` shape; returns the input clone unchanged when
  no rewrite applies (including the both-sides-negative case).

### Changed

- ``try_telescoping`` now calls ``normalise_add_neg_to_sub`` on
  ``Add`` inputs before the ``SUB`` head check.  Pure ``Sub`` and
  non-``Add`` shapes are untouched (zero cost).

### Added — tests

`tests/tests.rs` — 6 new ``phase46_*`` tests covering:

- ``Add(g(k+1), Neg(g(k)))`` closes to −1 (standard orientation).
- ``Add(Neg(g(k)), g(k+1))`` closes to −1 (operand-order swap).
- ``Add(g(k), Div(-1, k+1))`` closes to 1 (numerator-folded Neg,
  antisymmetric).
- ``Add(Div(-5, k+1), Div(5, k))`` closes to 5 (non-unit integer
  constant — the Python Phase 46 case).
- ``Add(Div(1/2, k), Div(-1/2, k+1))`` closes to 1/2
  (``IRNode::Rational`` numerator path).
- ``Add(Neg(a), Neg(b))`` intentionally stays unevaluated — no
  telescope to expose.

Full suite: **37 passed** (was 31; +6 net new).

### Still deferred

- ``Apart`` partial-fraction-decomposition handler.
- Transcendental limit-finder.

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
