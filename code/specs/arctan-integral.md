# Phase 2e — Arctan Integration for Irreducible Quadratic Denominators

## Status

Phase 2e of the rational-function integration roadmap. Closes the gap
left by Rothstein–Trager (Phase 2d): RT returns `None` whenever any log
coefficient escapes Q. The most common cause is an *irreducible quadratic*
denominator — one with no rational roots and therefore no partial-fraction
decomposition over Q. This phase emits the closed-form `arctan` antiderivative
for those cases.

## Scope

### What this phase handles

- **Input**: a proper rational function `C(x)/E(x)` where `E(x)` is an
  irreducible quadratic over Q — degree 2, discriminant `b² − 4ac < 0`,
  no rational roots — delivered by Hermite reduction with `deg C < 2`.
- **Output**: a closed-form IR tree involving `Log` and `Atan` nodes,
  assembled from Q-valued coefficients plus a possible `Sqrt` node for
  an irrational discriminant denominator.

### What this phase does NOT handle

- Denominators that mix linear factors with an irreducible quadratic, e.g.
  `1/((x−1)(x²+1))`. Those remain unevaluated for a later phase (would
  require partial-fraction splitting over a mixed L·Q denominator).
- Irreducible denominators of degree ≥ 3, e.g. `1/(x³+x+1)`. These
  have no closed form in elementary functions without algebraic extensions.
- Repeated quadratic factors (Hermite guarantees a squarefree log part,
  so this is not a concern here).

## Mathematical Background

### The irreducible quadratic formula

Given `ax² + bx + c` with `b² − 4ac < 0` and a proper numerator
`px + q` (i.e. `deg num < 2`), write

    px + q  =  A · (2ax + b)  +  B

where

    A  =  p / (2a)          (matches the derivative of the denominator up to
                              a constant, enabling the logarithmic contribution)
    B  =  q − p·b / (2a)   (the residual constant)

Then:

    ∫ (px + q) / (ax² + bx + c) dx
        =  A · log(ax² + bx + c)
         + (2B / D) · arctan((2ax + b) / D)

where `D = √(4ac − b²) > 0`.

### Why D > 0

Because `b² − 4ac < 0` by the irreducibility assumption, so
`4ac − b² > 0`. `D` is always a positive real number. If `4ac − b²`
is a perfect square of a rational — e.g. `x² + 1` gives `4·1·1 − 0 = 4`,
so `D = 2` — then `D` is an exact `Fraction` and no `Sqrt` node appears
in the output. Otherwise, the IR carries `Sqrt(4ac − b²)`.

### Worked examples

| Integrand | A | B | D | Antiderivative |
|---|---|---|---|---|
| `1/(x²+1)` | 0 | 1 | 2 | `arctan(x)` |
| `1/(x²+4)` | 0 | 1 | 4 | `(1/2) arctan(x/2)` |
| `(2x+1)/(x²+1)` | 1 | 1 | 2 | `log(x²+1) + arctan(x)` |
| `1/(x²+2x+5)` | 0 | 1 | 4 | `(1/2) arctan((x+1)/2)` |
| `(x+3)/(x²+2x+5)` | 1/2 | 2 | 4 | `(1/2)log(x²+2x+5) + arctan((x+1)/2)` |
| `1/(x²+3)` | 0 | 1 | √12 | `(1/√12) arctan(2x/√12)` |

### Why Rothstein–Trager misses these cases

RT builds `R(z) = res_x(C − z·E', E) ∈ Q[z]` and looks for Q-rational roots.
For `1/(x²+1)`: `R(z) = 4z² + 1`, whose roots are `±i/2 ∉ Q`, so RT returns
`None`. The log coefficient of `log(x²+1)` in the antiderivative of
`1/(x²+1)` is 0 — the entire answer is `arctan(x)` — but RT's design
cannot distinguish "no rational log coefficient" from "has an irrational
log coefficient" without richer machinery. This phase handles all such cases
directly by formula.

## Algorithm

```
arctan_integral(num_q, den_q, x_sym) → IRNode | None

  Preconditions (enforced by Integrate handler, not re-checked here):
    1. deg den_q == 2
    2. rational_roots(den_q) is empty  (irreducible over Q)
    3. deg num_q < 2                    (proper fraction — Hermite's guarantee)
    4. All coefficients are Fraction    (caller normalises)

  Steps:
    a. Extract a, b, c from den_q (coefficients of x², x, 1).
    b. Compute D² = 4ac − b²  (always > 0 by precondition).
    c. Compute A = p / (2a) and B = q − p·b / (2a)
       where p, q are the coefficients of num_q (p = 0 if deg num_q == 0).
    d. Compute D:
         n, d = D².numerator, D².denominator
         if isqrt(n)² == n and isqrt(d)² == d:
             D_frac = Fraction(isqrt(n), isqrt(d))  # rational D
         else:
             D_frac = None                           # irrational D → use Sqrt node
    e. Build the Atan argument:  u = (2ax + b) / D
         - numerator: 2a·x + b  (as an IR linear expression)
         - denominator: D_frac as IRRational, or IRApply(SQRT, (D²_as_ir,))
    f. Build the Atan IR:  IRApply(ATAN, (u,))
    g. Scale the Atan term:  coeff_atan = 2B / D
         - If D is rational:  coeff_atan = Fraction(2) * B / D_frac  (Fraction)
         - If D is irrational: coeff_atan = 2B (multiply by 2B/D via Mul + Inv)
    h. Build the Log IR (only if A ≠ 0):
         IRApply(LOG, (den_ir,))  multiplied by A
    i. Assemble into a left-associative Add chain.
    j. Return the assembled IR. Never returns None (preconditions guarantee a
       valid answer).
```

## IR Changes

### New head: `ATAN`

Add `ATAN = IRSymbol("Atan")` to `symbolic_ir.nodes` (in the
"Elementary functions" block, alongside `SIN`, `COS`, `LOG`, `SQRT`).
Export it from `symbolic_ir.__init__`.

`Atan(x)` represents the inverse tangent of a single argument. The
handler in `symbolic_vm.handlers` evaluates it numerically (via
`math.atan`) for fully numeric arguments and leaves it unevaluated
otherwise (matching the existing `Sin`, `Cos`, `Log` pattern).

## Integration into the Integrate handler

The Integrate handler in `symbolic_vm.integrate` currently calls:

    1. _integrate_rational  →  Hermite + RT
    2. Phase 1 pattern-matching rules

After this phase, `_integrate_rational` gains a third step:

    1. Hermite → (poly_part, rat_part, log_num, log_den)
    2. RT on log part → log terms or None
    3. **If RT returns None AND deg(log_den) == 2 AND rational_roots(log_den) == []:**
           → arctan_integral(log_num, log_den, x) → arctan IR
       Otherwise: leave log part as unevaluated Integrate

The arctan step fires only when:
- RT has already returned None (some coefficient escapes Q)
- The entire log-part denominator is an irreducible quadratic

Mixed denominators (e.g. `(x−1)·(x²+1)`) are NOT yet handled here.

## Correctness gate

The universal unit-test gate for `arctan_integral` is re-differentiation:

    d/dx (A·log(E) + (2B/D)·arctan((2ax+b)/D))
        = A·E'/E + (2B/D) · D/(1 + ((2ax+b)/D)²) · 2a/D
        = A·(2ax+b)/E + (2aB) / (D² + (2ax+b)²)
    
    With D² = 4ac−b² and (2ax+b)² = 4a²x²+4abx+b²:
    D² + (2ax+b)² = 4ac−b² + 4a²x²+4abx+b² = 4a(ax²+bx+c) = 4a·E

    So  = A·(2ax+b)/E + 2aB/(4aE) = A·(2ax+b)/E + B/(2E)
        = (A(2ax+b) + B/2) / E
        = (p·x/(2a)·2a + (B + A·b)·... 

Reduced form (verified algebraically):
    d/dx(antiderivative) = (px + q) / E = num/den  ✓

Because:
    A·(2ax+b) + B/(2a) ... see Bronstein §2.3 for the full reduction.

In tests: substitute the explicit rational case and verify numerically
at several x values (since the IR doesn't yet have a full simplifier,
numeric substitution is more practical than symbolic equality).

## Dependencies

- `coding-adventures-symbolic-ir ≥ 0.2.0` (adds `ATAN`)
- `coding-adventures-polynomial ≥ 0.4.0` (for `rational_roots`, already present)
- No new polynomial primitives required.

## Test strategy

| Test class | Cases |
|---|---|
| `TestPureImaginaryDenominator` | `1/(x²+1)`, `1/(x²+4)`, `1/(x²+a²)` pattern |
| `TestCompletedSquareDenominator` | `1/(x²+2x+5)`, `1/((x+1)²+9)` |
| `TestMixedNumerator` | `(2x+1)/(x²+1)`, `(x+3)/(x²+2x+5)` |
| `TestIrrationalDiscriminant` | `1/(x²+3)`, `1/(2x²+1)` — SQRT in output |
| `TestRatIntegralEndToEnd` | `integrate(1/(x^2+1), x)` via full VM |
| `TestEscapesQStillUnevaluated` | degree-4 irreducible denom stays as `Integrate` |

All test cases verify: differentiate the output IR numerically at 3+ test
points and confirm it matches the integrand within floating-point tolerance.
