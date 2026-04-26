# cas-factor — Polynomial Factorization over Q and Z

> **Status**: New spec. Implements the `Factor` head: decompose a
> polynomial into a product of irreducible factors over Q (or Z).
> Parent: `symbolic-computation.md`. Extends `polynomial.md`.

## Why this package exists

`polynomial` already supports squarefree factorization via Yun's
algorithm — it splits a polynomial into squarefree layers but does not
factor each layer into irreducibles. Real factoring (`(x^2 - 1) →
(x - 1)(x + 1)`) is a much bigger problem and deserves its own home
to keep `polynomial` lean.

This package is what backs `factor(x)` in any CAS frontend.

## Reuse story

Polynomial factoring is universal — Maxima's `factor`, Mathematica's
`Factor[]`, SymPy's `factor`, Maple's `factor` all use the same
underlying algorithms (Berlekamp–Zassenhaus over Z, then Hensel lift,
plus Kronecker for small inputs). One implementation serves them all.

## Scope

In:

- `Factor(expr, x)` — factor a univariate polynomial in `x` over Z.
- `Factor(expr)` — auto-detect the variable.
- Algorithms in increasing order of capability:
  - **Phase 1**: Squarefree (already in `polynomial`).
  - **Phase 2**: Rational-root test for linear factors.
  - **Phase 3**: Kronecker's method for small-degree polynomials
    (works for any polynomial in Z[x] but exponential in degree —
    practical only up to degree 6 or so).
  - **Phase 4**: Berlekamp factorization mod p, then Zassenhaus
    combinatorial reconstruction with Hensel lifting. The
    industry-standard approach.

Out:

- Multivariate factoring — future package.
- Algebraic-number factoring (factor over `Q[sqrt(2)]` etc.) — future.
- Numerical root-finding — that's `cas-solve`'s territory.

## Public interface

```python
from cas_factor import factor_univariate, register_handlers

factors = factor_univariate((-1, 0, 1))
# returns: [(coefficients_of_factor_1, multiplicity_1), ...]
# (-1, 0, 1) is x^2 - 1 → [((-1, 1), 1), ((1, 1), 1)]   i.e. (x-1)(x+1)

# As an IR head: Factor(x^2 - 1) -> (x - 1)*(x + 1)
```

The MACSYMA-facing entry point goes through the `Factor` head, which
the `register_handlers` function installs on a backend. The handler:

1. Convert IR → polynomial (via `polynomial-bridge`).
2. Call `factor_univariate`.
3. Convert each irreducible factor back to IR.
4. Multiply them with `Mul` and exponentiate by their multiplicity.

## Algorithm — Phase 1 to 4

**Phase 1 (squarefree, free)**: `polynomial.squarefree(p)` already
gives `p = c · s_1 · s_2^2 · ... · s_k^k`. Each `s_i` is squarefree.

**Phase 2 (rational root test)**: For each squarefree `s_i`, find
linear factors via the Rational Root Theorem: any rational root
`p/q` must have `p | s_i[0]` and `q | s_i[deg]`. Test each candidate
with Horner; on a hit, divide out the factor.

**Phase 3 (Kronecker)**: For polynomials of degree `n`, evaluate at
`n+1` integer points; the candidate factors of each value generate a
finite candidate set for factor coefficients. Try each combination.
Slow but complete.

**Phase 4 (Berlekamp + Zassenhaus + Hensel)**: Pick a small prime `p`
that doesn't divide the leading coefficient. Factor `s_i mod p` via
Berlekamp's algorithm (which is fast in `GF(p)[x]`). Hensel-lift the
mod-p factors to mod-`p^k` for `k` large enough. Combinatorially
re-multiply them in groups to find the true factors over Z.

The package starts at Phase 2 and graduates upward. Phase 4 is the
production target.

## Heads added

| Head        | Arity | Meaning                                  |
|-------------|-------|------------------------------------------|
| `Factor`    | 1–2   | `Factor(expr)` or `Factor(expr, var)`.   |
| `Irreducible` | 1   | Predicate: is this polynomial prime?     |

## Test strategy

- `Factor(x^2 - 1) = (x - 1)(x + 1)`.
- `Factor(x^2 + 2x + 1) = (x + 1)^2`.
- `Factor(x^3 - x) = x(x - 1)(x + 1)`.
- `Factor(x^4 + 1)` — irreducible over Q.
- `Factor(x^4 - 1) = (x - 1)(x + 1)(x^2 + 1)`.
- `Factor(2*x^2 + 4*x + 2) = 2*(x + 1)^2` (content extracted).
- High degree (degree 8+) handled by Phase 4.
- Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-factor/
  src/cas_factor/
    __init__.py
    rational_roots.py   # Phase 2
    kronecker.py        # Phase 3
    berlekamp.py        # Phase 4: factor in GF(p)[x]
    hensel.py           # Phase 4: lift mod-p to mod-p^k
    zassenhaus.py       # Phase 4: combinatorial recombination
    factor.py           # orchestrates Phase 1 → 4
    py.typed
  tests/
    test_rational_roots.py
    test_kronecker.py
    test_berlekamp.py
    test_factor_orchestration.py
```

Dependencies: `coding-adventures-symbolic-ir`,
`coding-adventures-polynomial`,
`coding-adventures-polynomial-bridge`.

## Future extensions

- Multivariate Kronecker substitution.
- Factoring over `Q[α]` for algebraic `α`.
- Trager's algorithm for symbolic integration of algebraic functions
  (which uses factorization over algebraic extensions).
