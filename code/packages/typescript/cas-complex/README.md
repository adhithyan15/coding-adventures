# cas-complex (TypeScript)

Pure TypeScript complex-number arithmetic over symbolic IR expressions. The
package mirrors the Rust `cas-complex` crate and runs without native bindings,
so browser-side CAS paths can normalize complex expressions directly.

## Operations

| Function | Description |
|---|---|
| `complexNormalize(z)` | Rewrite `z` into canonical `a + b*I` form |
| `splitComplex(z)` | Return `[real, imag]` parts as real-valued IR nodes |
| `realPart(z)` | Extract real part `Re(z)` |
| `imagPart(z)` | Extract imaginary part `Im(z)` |
| `conjugate(z)` | Complex conjugate: `a + b*I -> a - b*I` |
| `modulus(z)` | Modulus `|z| = sqrt(a^2 + b^2)` as an IR float |
| `argument(z)` | Argument `atan2(b, a)` as an IR float |
| `complexPow(z, n)` | Integer power via De Moivre's theorem |

## Normalization

`complexNormalize` handles `Add`, `Sub`, `Mul`, `Neg`, and `Pow(I, n)` for
integer `n`. Numeric literals are pure real values; symbols other than `I` are
treated as opaque real atoms.

Zero parts are suppressed: `0 + 2*I` becomes `Mul(2, I)`, `3 + 0*I` becomes
`3`, and `0 + 1*I` becomes `I`.
