# cas-factor

Univariate polynomial factoring over Q. Phase 1 ships a small but
complete subset:

- **Content extraction** — pull out the integer GCD of all coefficients,
  reducing ``2x² + 4x + 2`` to ``2 · (x² + 2x + 1)``.
- **Rational-root test** — by the Rational Root Theorem, any rational
  root ``p/q`` of an integer polynomial has ``p | a₀`` and ``q | aₙ``.
  We enumerate the candidates, test each, and divide out the factor on
  every hit.

The combination handles any polynomial whose irreducible factors are
all linear over Q — which covers the bulk of high-school and early
undergraduate-textbook examples.

## Phase 2 (deferred)

For irreducible quadratics, cubics, and higher, this Phase 1 leaves
them intact. Phase 2 will add Berlekamp factorization mod p plus
Hensel lifting and Zassenhaus combinatorial reconstruction — the
production-grade approach for arbitrary degree.

## API

```python
from cas_factor import factor_integer_polynomial

# Polynomial as a coefficient list (constant term first):
#   x^2 - 1  →  [-1, 0, 1]
content, factors = factor_integer_polynomial([-1, 0, 1])
# content = 1, factors = [([-1, 1], 1), ([1, 1], 1)]
# meaning  1 · (x - 1)^1 · (x + 1)^1
```

Each factor is ``(coefficient_list, multiplicity)``.

## Reuse story

Universal across CAS frontends — backs ``factor()`` in Maxima,
``Factor[]`` in Mathematica, ``factor`` in Maple, ``factor`` in
SymPy. Multivariate factoring lives in a separate package.

## Dependencies

- `coding-adventures-symbolic-ir`
