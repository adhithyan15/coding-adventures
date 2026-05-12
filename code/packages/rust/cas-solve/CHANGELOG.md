# Changelog — cas-solve (Rust)

## Unreleased

### Added

- `quartic` module: `solve_quartic(a, b, c, d, e) -> SolveResult` — solves
  quartic equations via rational-root deflation, biquadratic solving, and a
  bounded Ferrari fallback through the cubic solver.
- Quartic integration tests for cubic delegation, exact rational roots, zero
  root deduplication, biquadratic complex roots, Ferrari complex roots, and
  unevaluated resolvent fallback.
- `cubic` module: `solve_cubic(a, b, c, d) -> SolveResult` — solves cubic
  equations via rational-root deflation plus a focused Cardano symbolic
  fallback matching the Python reference's tested behavior.
- `CBRT = "Cbrt"` head name constant for cubic Cardano expressions.
- Cubic integration tests for quadratic delegation, exact rational roots,
  repeated roots, rational fraction roots, symbolic Cardano, and casus
  irreducibilis fallback.

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-solve` package.
- `frac` module: `Frac` — exact rational arithmetic with `i64` storage and
  `i128` intermediaries for overflow safety; implements `Add`, `Sub`, `Mul`,
  `Div`, `Neg`; `to_irnode()` converts to `IRNode::Integer` or `IRNode::Rational`.
- `linear` module: `solve_linear(a, b) -> SolveResult` — solves `a·x + b = 0`;
  returns `Solutions([x])`, `Solutions([])` (no solution), or `All`.
- `quadratic` module: `solve_quadratic(a, b, c) -> SolveResult` — solves
  `a·x² + b·x + c = 0` via the quadratic formula:
  - Perfect-square discriminant: rational roots.
  - Positive irrational discriminant: `Div(Add/Sub(-b, Sqrt(disc)), 2a)`.
  - Negative discriminant: complex roots `r ± k·%i` (Maxima convention).
  - `a = 0` fallback: delegates to `solve_linear`.
- `SolveResult` enum: `Solutions(Vec<IRNode>)` or `All`.
- `SOLVE`, `NSOLVE`, `ROOTS` head name constants.
- `I_UNIT = "%i"` imaginary unit symbol (Maxima/MACSYMA convention).
- 17 integration tests + 3 doc-tests; all passing.
