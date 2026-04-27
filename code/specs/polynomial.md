# Polynomial — Univariate Polynomial Arithmetic

> **Status**: The package already exists at MA00 and powers `gf256` and
> `reed-solomon`. This spec documents its scope and the Phase 2 extensions
> that unlock symbolic integration (Hermite reduction, Rothstein–Trager).
> It is written as the authoritative spec for the package as a whole, not
> as a change log — see `CHANGELOG.md` for the delta.

## Why this package exists

`polynomial` is the shared foundation for two otherwise unrelated stacks:

- **Error-correcting codes** (`gf256`, `reed-solomon`) — polynomial
  arithmetic over GF(2^8) drives syndrome computation and the
  Berlekamp–Massey decoder.
- **Symbolic integration** (Phase 2+ of `symbolic-computation.md`) —
  Hermite reduction, Rothstein–Trager, and the Risch algorithm all
  require a real polynomial type with GCD, division, formal derivative,
  and squarefree factorization. Those operations are prohibitive in the
  raw symbolic IR — expression size blows up exponentially.

The two use-cases share the exact same tuple representation and the same
Euclidean algorithm. The only difference is the coefficient ring —
`gf256` uses `int` (mod-2 polynomials over GF(2)), symbolic integration
uses `fractions.Fraction` (Q[x]). Everything in this package is written
polymorphically so both consumers get the same code path.

## Representation

A polynomial is a tuple of coefficients from the constant term up:

    a_0 + a_1·x + a_2·x^2 + … + a_n·x^n   →   (a_0, a_1, …, a_n)

- Trailing zeros are stripped on every operation (`normalize`). So
  `(1, 0, 0)` and `(1,)` are the same value.
- The zero polynomial is `()`, with degree `-1` by convention.
- Coefficient type is whatever the caller passes in — `int`, `float`,
  `fractions.Fraction`, GF(2^8) element, etc. The package never coerces
  the type; it only uses `+`, `-`, `*`, `/`, and comparison with `0`.

### Exact vs. approximate coefficients

The original consumers (`gf256`, `reed-solomon`) use exact integer /
field arithmetic. Symbolic integration needs exact rational arithmetic.
The package therefore **never seeds accumulators with `0.0`** — it seeds
with the integer `0`, which is the additive identity for every numeric
type we care about. This keeps `multiply((Fraction(1, 2),), (Fraction(1, 3),))`
returning `(Fraction(1, 6),)` instead of silently collapsing to `(0.166…,)`.
A CAS that silently drops to floats is a CAS that silently produces
wrong answers.

## Operations

### Core (pre-existing)

| Operation              | Function                   | Notes                                          |
|------------------------|----------------------------|------------------------------------------------|
| Strip trailing zeros   | `normalize(p)`             |                                                |
| Degree                 | `degree(p)`                | `-1` for the zero polynomial                   |
| Zero polynomial        | `zero()`                   | `()`                                           |
| One polynomial         | `one()`                    | `(1,)`                                         |
| Addition               | `add(a, b)`                |                                                |
| Subtraction            | `subtract(a, b)`           |                                                |
| Multiplication         | `multiply(a, b)`           | Polynomial convolution                         |
| Division with remainder| `divmod_poly(a, b)`        | Raises `ValueError` on `b = 0`                 |
| Quotient only          | `divide(a, b)`             |                                                |
| Remainder only         | `mod(a, b)`                |                                                |
| Evaluation             | `evaluate(p, x)`           | Horner's method; returns coefficient type      |
| GCD (Euclidean)        | `gcd(a, b)`                | Raw Euclidean — not monic-normalized           |

### Calculus & factorization extensions (Phase 2a)

| Operation              | Function                   | Notes                                                        |
|------------------------|----------------------------|--------------------------------------------------------------|
| Formal derivative      | `deriv(p)`                 | `d/dx` — `(a_0, a_1, a_2) → (a_1, 2·a_2)`                    |
| Monic normalization    | `monic(p)`                 | Divide through by leading coefficient. Requires a field.     |
| Squarefree factorization | `squarefree(p)`          | Yun's algorithm. Returns `[s_1, s_2, …, s_k]` with `p = c · s_1 · s_2^2 · … · s_k^k`. Each factor is monic, squarefree, and pairwise coprime. |

`squarefree` is the factorization primitive Hermite reduction needs: it
splits the denominator of a rational integrand into squarefree layers
without requiring irreducible factorization (which over Q[x] is a much
bigger problem — Berlekamp, Zassenhaus, LLL).

## Examples

### Integer / real coefficients (the original use-case)

```python
from polynomial import add, multiply, evaluate, gcd, divmod_poly

p = (1, 2, 3)                     # 1 + 2x + 3x²
evaluate(p, 2)                    # → 17   (1 + 4 + 12)
multiply((1, 1), (2, 1))          # → (2, 3, 1)   (x+1)(x+2)

q, r = divmod_poly((5, 1, 3, 2), (2, 1))
# q = (3, -1, 2), r = (-1,)
```

### Rational coefficients (Phase 2a — CAS integration)

```python
from fractions import Fraction
from polynomial import deriv, monic, squarefree, multiply

# (x - 1)·(x - 2)² — rewritten with Fraction coefficients
x_minus_1 = (Fraction(-1), Fraction(1))
x_minus_2 = (Fraction(-2), Fraction(1))
p = multiply(x_minus_1, multiply(x_minus_2, x_minus_2))

deriv(p)                          # formal derivative, still exact
monic(p)                          # leading coefficient normalised to 1
factors = squarefree(p)           # [x - 1, x - 2] (with multiplicities encoded by position)
# factors[0] is the product of simple roots, factors[1] is the product of double roots, …
```

## Non-Goals

- **Factorization into irreducibles** over Q[x] — Berlekamp, Zassenhaus,
  LLL. Risch's transcendental case doesn't need it (Rothstein–Trager
  sidesteps factoring via a resultant); we'll reassess if the algebraic
  case lands later.
- **Multivariate polynomials.** Separate package, much larger design.
- **Finite-field coefficients beyond the existing GF(2^8) use-case.**
  If we ever need factoring mod p, it gets its own module.
- **Numerical root finding** — CAS work is symbolic.
- **Sparse representations.** Integrands a human wrote are low-degree
  and dense; a tuple of coefficients is easier to reason about.

## Test Strategy

- Dense coverage on every operation — neutral elements, commutativity,
  associativity, distributivity spot-checks.
- **Fraction workflow**: multiply, divide, and evaluate a polynomial
  with `Fraction` coefficients end-to-end and confirm the result is
  still `Fraction` (no silent float promotion).
- `deriv`: `(a, b, c, d) → (b, 2c, 3d)`; derivative of a constant is `()`.
- `monic`: leading coefficient becomes `1`; zero polynomial passes through.
- `squarefree`:
  - `(x - 1)·(x - 2)²·(x - 3)³` → `[x-1, x-2, x-3]` (by position).
  - Squarefree input returns `[p]` unchanged (up to monic scale).
  - `gcd(p, p') = 1` path (fully squarefree) doesn't break.
- Backwards compatibility: the existing gf256 / reed-solomon test
  suites continue to pass.
- Coverage target ≥ 95% (leaf library, no I/O).
