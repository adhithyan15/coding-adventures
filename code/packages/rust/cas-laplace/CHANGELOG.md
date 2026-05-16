# Changelog

## 0.2.0 — 2026-05-16

**Extended ILT engine: complex conjugate poles, repeated poles, improper
fractions.  Extended forward table: t^n·sin(ωt) and t^n·cos(ωt) for n = 2, 3.**

### `lib.rs` — partial-fraction decomposition engine

`inverse_lookup` now falls through to a full partial-fraction engine
`inverse_pf` when direct pattern matching fails.  Three new capabilities:

1. **Improper fractions** — polynomial long division extracts the quotient
   `P(s)`.  A constant quotient contributes a `DiracDelta(t)` term.

2. **Repeated rational poles** — formal power-series expansion around the
   pole: shift `s → r + t`, divide out `t^m`, and read off the first `m`
   Taylor coefficients.  All arithmetic is exact, using a `Frac` struct
   backed by `i64`.

3. **Irreducible quadratic factor** — after extracting all rational-root
   factors, a remaining degree-2 denominator with `b²−4c < 0` is handled
   by completing the square `(s+α)²+β²`, yielding
   `exp(−αt)·cos(βt)` and `exp(−αt)·sin(βt)`.  When β is irrational a
   `Sqrt(β²)` IR node keeps the result exact.

New internal infrastructure: `Frac` struct with i64 arithmetic;
`Poly = Vec<Frac>`; `ir_to_rational`; `poly_shift`; `power_series_coeffs`;
`compute_repeated_residues`; `ilt_irreducible_quad`; `inverse_pf`;
`rational_roots`; `extract_all_rational_roots`.

Examples now evaluating:
- `ilt(1/(s^2+2*s+2), s, t)` → `Mul(Exp(Neg(t)), Sin(t))`
- `ilt(1/(s-2)^2, s, t)` → `Mul(t, Exp(Mul(2, t)))`
- `ilt(1/(s*(s^2+1)), s, t)` → `Add(UnitStep(t), Mul(-1, Cos(t)))`

### `lib.rs` — t^n·trig forward transforms for n = 2, 3

Added `match_tn_times_trig` pattern matcher and inline transform builders
for `t^n·sin(ωt)` and `t^n·cos(ωt)` with n = 2, 3.

| f(t)        | F(s) = L{f}(s)                  |
|-------------|----------------------------------|
| t²·sin(ωt)  | 2ω(3s²−ω²) / (s²+ω²)³           |
| t²·cos(ωt)  | 2s(s²−3ω²) / (s²+ω²)³           |
| t³·sin(ωt)  | 24ωs(s²−ω²) / (s²+ω²)⁴          |
| t³·cos(ωt)  | 6(s⁴−6s²ω²+ω⁴) / (s²+ω²)⁴      |

For n ≥ 4 the pattern matches but returns `None`, falling through to
unevaluated `Laplace(f, t, s)`.

### Test count

Tests expanded from 6 to 15.

## 0.1.0

- Added a pure Rust table-driven Laplace and inverse Laplace transform package.
