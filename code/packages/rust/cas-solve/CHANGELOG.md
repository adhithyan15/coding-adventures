# Changelog — cas-solve (Rust)

## Unreleased

### Added

- `transcendental` module: `try_solve_transcendental(eq, variable)` — direct
  `f(linear) = constant` solving for trig, exponential/logarithmic, and
  hyperbolic functions with symbolic inverse and periodic-family output.
- Transcendental integration tests for exp/log, trig periodic families,
  hyperbolic inverses, reversed equations, bare zero-form expressions, and
  unsupported nested/non-variable cases.
- `linear_system` module: `solve_linear_system(equations, variables)` — exact
  Gaussian elimination with `Equal(lhs, rhs)` normalization, zero-form
  equations, and `Rule(var, value)` IR output.
- Linear-system integration tests for integer, rational, 3x3, zero-form,
  singular, non-linear, and wrong-size systems.
- `inequality` module: `try_solve_inequality(ineq, variable)` — one-variable
  polynomial inequality solving for `Less`, `LessEqual`, `Greater`, and
  `GreaterEqual` IR nodes up to quartic degree.
- Inequality integration tests for exact rational boundaries, interval
  conditions, all-real sentinels, numeric irrational boundaries, and
  non-polynomial rejection.
- `numeric` module: `Complex`, `nsolve_poly`, `roots_to_ir`, and
  `nsolve_fraction_poly` — pure Rust Durand-Kerner numeric polynomial
  root-finding plus `%i`-aware symbolic IR conversion.
- Numeric solver integration tests for linear, quadratic, cubic, quintic,
  constant-polynomial, complex-root, and fraction-coefficient cases.
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
