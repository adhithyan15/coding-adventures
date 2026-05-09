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

`solveQuadratic` returns rational roots when the discriminant is a perfect
square, symbolic `Sqrt` roots for positive irrational discriminants, and `%i`
complex roots for negative discriminants.
