# cas-complex

Complex number IR support for the symbolic computation substrate.

Provides `ImaginaryUnit` as a pre-bound symbol satisfying `ImaginaryUnit² = -1`,
arithmetic normalization to rectangular form `a + b·i`, and utility heads for
complex decomposition and transformation.

## Heads

| Head         | Meaning                                           |
|--------------|---------------------------------------------------|
| `Re`         | Real part of `a + b·i` → `a`                     |
| `Im`         | Imaginary part of `a + b·i` → `b`                |
| `Conjugate`  | Complex conjugate: `a + b·i → a - b·i`           |
| `Abs`        | Complex modulus: `√(a² + b²)`                    |
| `Arg`        | Principal argument: `arctan(b/a)`                 |
| `RectForm`   | Rewrite as `a + b·ImaginaryUnit`                  |
| `PolarForm`  | Rewrite as `r·Exp(ImaginaryUnit·θ)`               |

All heads are language-neutral and live on `SymbolicBackend`.
