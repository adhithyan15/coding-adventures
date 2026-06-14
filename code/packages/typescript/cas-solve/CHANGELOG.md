# Changelog

## Unreleased

- Add `trySolveTranscendental` for direct Phase 26-style transcendental
  equations `f(linear) = constant` across trig, exp/log, and hyperbolic heads.
- Add `trySolveInequality` for Phase 27-style polynomial inequality solving
  over rational univariate polynomials up to degree 4.
- Add `solveLinearSystem` with exact Gaussian elimination, `Equal(lhs, rhs)`
  normalization, zero-form equations, and `Rule(var, value)` IR output.
- Add pure TypeScript Durand-Kerner numeric polynomial solving plus IR
  conversion helpers for numeric roots.
- Add pure TypeScript quartic solving parity with rational-root deflation,
  biquadratic solving, and Ferrari factorization through the cubic solver.
- Add pure TypeScript cubic closed-form solving parity for rational roots,
  repeated roots, quadratic delegation, and Cardano symbolic fallback.

## 0.1.0

- Port exact fractions, linear solving, and quadratic solving to pure
  TypeScript.
