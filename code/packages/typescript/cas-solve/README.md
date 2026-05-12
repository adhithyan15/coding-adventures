# cas-solve (TypeScript)

Pure TypeScript closed-form equation solving over rational coefficients. This
package mirrors the current Rust `cas-solve` crate and runs without native
bindings.

## Operations

| Function | Description |
|---|---|
| `Frac` | Exact reduced rational helper |
| `solveLinear(a, b)` | Solve `a*x + b = 0` |
| `solveQuadratic(a, b, c)` | Solve `a*x^2 + b*x + c = 0` |
| `solveCubic(a, b, c, d)` | Solve `a*x^3 + b*x^2 + c*x + d = 0` |
| `solveQuartic(a, b, c, d, e)` | Solve `a*x^4 + b*x^3 + c*x^2 + d*x + e = 0` |
| `solveLinearSystem(equations, variables)` | Solve exact square linear systems and return `Rule(var, value)` nodes |
| `trySolveInequality(ineq, variable)` | Solve polynomial inequalities up to degree 4 and return interval conditions |
| `trySolveTranscendental(eq, variable)` | Solve direct `f(linear) = constant` transcendental equations |
| `nsolvePoly(coeffs)` | Numerically solve a polynomial via Durand-Kerner |
| `rootsToIr(roots)` / `nsolveFractionPoly(coeffs)` | Convert numeric roots to symbolic IR |

`solveQuadratic` returns rational roots when the discriminant is a perfect
square, symbolic `Sqrt` roots for positive irrational discriminants, and `%i`
complex roots for negative discriminants.

`solveCubic` first applies the rational-root theorem and deflates to the
quadratic solver so exact rational and repeated roots stay exact. When no
rational root exists it follows the Python package's Cardano behavior, returning
symbolic `Cbrt`/`Sqrt` expressions for the one-real-root case and an empty
solution list for casus irreducibilis.

`solveQuartic` follows the Python package's quartic path: rational-root
deflation first, biquadratic solving for even quartics, and Ferrari
factorization when the resolvent cubic has a usable rational root.

`solveLinearSystem` accepts `Equal(lhs, rhs)` nodes or zero-form expressions,
linearizes them over the provided variables, and solves square systems with
exact Gaussian elimination. It returns `null` for non-linear, singular, empty,
or non-square systems.

`trySolveInequality` accepts `Less`, `Greater`, `LessEqual`, or `GreaterEqual`
IR nodes, normalizes `lhs op rhs` to a univariate rational polynomial in the
requested variable, and returns interval predicates such as `Less(x, a)`,
`GreaterEqual(x, a)`, or bounded `And(...)` ranges. Unsupported non-polynomial
inputs return `null`.

`trySolveTranscendental` accepts `Equal(lhs, rhs)` or a bare expression treated
as equal to zero. It handles direct equations where one side is a supported
transcendental function of a linear expression in the requested variable and
the other side is constant with respect to that variable. The first slice covers
`Sin`, `Cos`, `Tan`, `Exp`, `Log`, `Sinh`, `Cosh`, and `Tanh`.

`nsolvePoly` accepts real or complex coefficients in descending degree order,
normalizes by the leading coefficient, and returns all roots as `{ re, im }`
objects. `nsolveFractionPoly` is a convenience wrapper for exact `Frac`
coefficients that returns IR float/complex roots.
