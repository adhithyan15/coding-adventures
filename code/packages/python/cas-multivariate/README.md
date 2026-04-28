# cas-multivariate

Multivariate polynomial operations and Gröbner bases for the MACSYMA CAS system.

## What it does

This package implements:

- **Gröbner basis computation** (Buchberger's algorithm) over Q[x₁, …, xₙ]
- **Polynomial reduction** (multivariate division with remainder)
- **Ideal solving** — solve polynomial systems via lex Gröbner basis + back-substitution

All arithmetic is exact (uses Python's `fractions.Fraction`).

## How it fits in the stack

```
symbolic-ir  ←  defines IR nodes + GROEBNER/POLY_REDUCE/IDEAL_SOLVE heads
   ↑
cas-multivariate  ←  this package: pure math + IR handlers
   ↑
symbolic-vm  ←  registers the handlers in build_cas_handler_table()
   ↑
macsyma-runtime  ←  maps "groebner"/"poly_reduce"/"ideal_solve" to IR heads
```

## Usage

### Direct (no VM)

```python
from fractions import Fraction
from cas_multivariate.polynomial import MPoly
from cas_multivariate.groebner import buchberger
from cas_multivariate.solve import ideal_solve

F = Fraction

# Represent x + y - 1 and x - y in Q[x, y]
f1 = MPoly({(1,0): F(1), (0,1): F(1), (0,0): F(-1)}, 2)  # x + y - 1
f2 = MPoly({(1,0): F(1), (0,1): F(-1)}, 2)                # x - y

# Compute Gröbner basis
G = buchberger([f1, f2], order="grlex")

# Solve the system
solutions = ideal_solve([f1, f2])
# → [[Fraction(1, 2), Fraction(1, 2)]]
```

### MACSYMA surface syntax (via VM + macsyma-runtime)

```
groebner([x^2+y-1, x+y^2-1], [x, y])   → Gröbner basis as List(...)
poly_reduce(x^2, [x-1], [x])            → 1
ideal_solve([x+y-1, x-y], [x, y])       → [[x=1/2, y=1/2]]
```

## Monomial orderings

Three orderings are supported:

- **grlex** (default): graded lexicographic — standard for Buchberger
- **lex**: lexicographic — used for back-substitution in `ideal_solve`
- **grevlex**: graded reverse lexicographic

## Scope and limits

- Coefficients: rationals Q only (no floating-point, no modular arithmetic)
- Max total degree of any polynomial: 8
- Max number of variables: 4 (practical)
- Safety cap: Gröbner basis limited to 50 elements

## Mathematical background

A **Gröbner basis** is a special generating set for a polynomial ideal
such that the remainder on division is well-defined.  Buchberger's
algorithm (1965) computes one from any generating set by repeatedly
adding S-polynomials that don't reduce to zero.

The **lex order** Gröbner basis has a triangular structure (like Gaussian
elimination for nonlinear systems) that enables back-substitution to find
exact solutions.
