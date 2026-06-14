# Changelog

## 0.2.0 — 2026-05-16

**Extended ILT engine: complex conjugate poles, repeated poles, improper
fractions.  Extended forward table: t^n·sin(ωt) and t^n·cos(ωt) for n = 2, 3.**

### `index.ts` — partial-fraction decomposition engine

`inverseLookup` now falls through to a full partial-fraction engine
`inversePF` when direct pattern matching fails.  Three new capabilities:

1. **Improper fractions** — polynomial long division extracts the quotient
   `P(s)`.  A constant quotient contributes a `DiracDelta(t)` term.

2. **Repeated rational poles** — formal power-series expansion around the
   pole: shift `s → r + t`, divide out `t^m`, and read off the first `m`
   Taylor coefficients of the reduced function.  All arithmetic is exact,
   using a `Frac` type backed by `bigint`.

3. **Irreducible quadratic factor** — after extracting all rational-root
   factors, a remaining degree-2 denominator with `b²−4c < 0` is handled
   by completing the square `(s+α)²+β²` and matching against
   `A*(s+α)/((s+α)²+β²)` and `β/((s+α)²+β²)`, yielding
   `exp(−αt)·cos(βt)` and `exp(−αt)·sin(βt)` respectively.  When β is
   irrational a `Sqrt(β²)` node is built so the output remains exact.

New internal infrastructure: `Frac` type with exact bigint arithmetic;
`Poly = Frac[]` polynomial arithmetic; `irToRational`; `polyShift`;
`powerSeriesCoeffs`; `computeRepeatedResidues`; `iltIrreducibleQuad`;
`inversePF`; `rationalRoots`; `extractAllRationalRoots`.

Examples now evaluating:
- `ilt(1/(s^2+2*s+2), s, t)` → `Mul(Exp(Neg(t)), Sin(t))`
- `ilt(s/(s^2+2*s+2), s, t)` → `Add(Mul(Exp(Neg(t)),Cos(t)), Mul(-1,Mul(Exp(Neg(t)),Sin(t))))`
- `ilt(1/(s-2)^2, s, t)` → `Mul(t, Exp(Mul(2, t)))`
- `ilt(1/(s*(s^2+1)), s, t)` → `Add(UnitStep(t), Mul(-1, Cos(t)))`

### `index.ts` — t^n·trig forward transforms for n = 2, 3

Added `matchTnTimesTrig` pattern recognizer and inline transform builders
for `t^n·sin(ωt)` and `t^n·cos(ωt)` with n ≥ 2.

Closed-form formulas:

| f(t)        | F(s) = L{f}(s)                  |
|-------------|----------------------------------|
| t²·sin(ωt)  | 2ω(3s²−ω²) / (s²+ω²)³           |
| t²·cos(ωt)  | 2s(s²−3ω²) / (s²+ω²)³           |
| t³·sin(ωt)  | 24ωs(s²−ω²) / (s²+ω²)⁴          |
| t³·cos(ωt)  | 6(s⁴−6s²ω²+ω⁴) / (s²+ω²)⁴      |

For n ≥ 4 the pattern matches but returns `undefined`, falling through to
unevaluated `Laplace(f, t, s)`.

### Test count

Tests expanded from 10 to 35.

## 0.1.0

- Added a pure TypeScript table-driven Laplace and inverse Laplace transform package.
