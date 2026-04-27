# Changelog — cas-trig (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial implementation of symbolic trigonometry over the CAS IR.
- `constants` module: re-exports `SIN`, `COS`, `TAN`, `SQRT`, `ATAN`,
  `ASIN`, `ACOS` from `symbolic_ir`; defines `PI_SYMBOL = "Pi"` and
  `E_SYMBOL = "E"`.
- `special` module: exact algebraic values for sin, cos, tan at rational
  multiples of π with denominator in `{1, 2, 3, 4, 6}`:
  - `sin_at_pi_multiple(num, den)` — covers all 16 canonical angles in [0, 2π)
  - `cos_at_pi_multiple(num, den)` — same coverage
  - `tan_at_pi_multiple(num, den)` — returns `None` at poles (π/2, 3π/2)
  - `reduce_pi_fraction(num, den)` — normalises any rational multiple to [0, 2)
    using Euclidean remainder; handles negative inputs and values > 2π.
  - Surd representations: `√2/2 = Mul(1/2, Sqrt(2))`, `√3/2 = Mul(1/2, Sqrt(3))`,
    `√3 = Sqrt(3)`, `1/√3 = Mul(1/3, Sqrt(3))`.
- `numeric` module: float evaluation for all trig functions:
  - `to_float` — converts `Integer`, `Float`, `Rational`, `Symbol("Pi")` to `f64`.
  - `sin_numeric`, `cos_numeric`, `tan_numeric`, `atan_numeric`,
    `asin_numeric`, `acos_numeric`.
  - `snap(v)` — snaps near-integers (within 1e-9) to exact `IRInteger`.
  - `tan_numeric` returns `Float(f64::INFINITY)` at poles so callers can
    return an unevaluated node.
- `simplify` module: main evaluation entry points with 3-tier dispatch:
  - `sin_eval`, `cos_eval`, `tan_eval`, `atan_eval`, `asin_eval`, `acos_eval`.
  - `trig_simplify(expr)` — recursive tree walker; evaluates every
    recognised trig node bottom-up.
  - `extract_pi_multiple(arg)` — recognises `Integer(0)`, `Symbol("Pi")`,
    `Mul(r, Pi)`, `Mul(Pi, r)`, and their `Neg(…)` wrappers as rational
    multiples of π.
- `expand` module: angle-addition formulas:
  - `expand_trig(expr)` — rewrites `sin/cos(a±b)`, `sin/cos(-a)`, and
    double-angle `sin/cos(2a)` throughout an expression tree.
  - Identities: `sin(a+b) = sin(a)cos(b)+cos(a)sin(b)`, etc.
  - Double-angle: `sin(2a) = 2sin(a)cos(a)`, `cos(2a) = cos²(a)−sin²(a)`.
- `reduce` module: power reduction:
  - `power_reduce(expr)` — rewrites `Pow(Sin(x), 2)` to
    `Mul(1/2, Sub(1, Cos(Mul(2, x))))` and `Pow(Cos(x), 2)` to
    `Mul(1/2, Add(1, Cos(Mul(2, x))))` throughout the tree.
  - Derivation: `cos(2x) = 1 - 2sin²(x)` and `cos(2x) = 2cos²(x) - 1`.
- 78 integration tests + 10 doc-tests; all passing, zero warnings.
  - Special-value coverage: all 16 canonical angles in [0, 2π) × 3 functions.
  - Numeric: float evaluation, rational args, near-integer snapping.
  - Periodicity: `sin(2π) = 0`, `cos(-π) = -1`, `sin(-π/2) = -1`.
  - Expansion: sum, difference, negation, double-angle (verified numerically).
  - Reduction: sin², cos², nested in Add (Pythagorean identity check).
- Depends only on `symbolic-ir`; no external crates.
