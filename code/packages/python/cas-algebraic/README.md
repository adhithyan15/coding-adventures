# cas-algebraic 0.1.0

Polynomial factoring over algebraic number fields Q[√d] for the MACSYMA CAS.

## What it does

Given a polynomial that is irreducible over Q, `cas-algebraic` determines whether
it factors over the algebraic extension Q[√d] = { p + q·√d | p,q ∈ Q } for a
user-specified square-free positive integer d.

```
factor(x^4 + 1, sqrt(2))   → (x^2 + sqrt(2)*x + 1) * (x^2 - sqrt(2)*x + 1)
factor(x^2 - 2, sqrt(2))   → (x - sqrt(2)) * (x + sqrt(2))
factor(x^2 - 5, sqrt(5))   → (x - sqrt(5)) * (x + sqrt(5))
```

## Mathematical background

The simplest algebraic extension of Q is formed by adjoining a root of
an irreducible quadratic.  For any square-free positive integer d, the field:

    Q[√d] = { a + b√d | a, b ∈ Q }

contains all rationals and additionally the irrational √d.  Polynomials that
are irreducible over Q may factor over this extended field.

### Pattern 1: Depressed quartics

For g = x⁴ + p·x² + q, we search for r, s ∈ Q such that:

    g = (x² + r√d·x + s)(x² − r√d·x + s)

Conditions: s = ±√q (q must be a perfect rational square) and
r = √((2s − p)/d) (must be a non-negative rational square).

**Example**: x⁴ + 1 over Q[√2]: p=0, q=1, s=1, r=√(2/2)=1.
Result: (x² + √2·x + 1)(x² − √2·x + 1).

### Pattern 2: Quadratic splitting

For g = x² + bx + c, the polynomial splits over Q[√d] when:

    (b² − 4c) / (4d) = β²  for some rational β

Then g = (x + b/2 − β√d)(x + b/2 + β√d).

**Example**: x² − 2 over Q[√2]: b=0, c=−2, disc=8, 8/(4·2)=1=1². Split: (x−√2)(x+√2).

## Stack position

```
cas-algebraic  (this package)
    ↓ uses
cas-factor         (integer polynomial factoring over Z)
symbolic-ir        (IR node types)
symbolic-vm        (registers AlgFactor handler)
macsyma-runtime    (exposes algfactor surface syntax)
```

## Usage

### Direct algebraic factoring

```python
from cas_algebraic import factor_over_extension
from fractions import Fraction

# x^4 + 1 over Q[√2]
factors = factor_over_extension([1, 0, 0, 0, 1], d=2)
# Returns list of AlgPoly factors
# Each factor: [(rational_part, radical_part), ...] in ascending degree
```

### MACSYMA surface syntax

After the runtime wires in the `algfactor` name (version 1.9.0+):

```
algfactor(x^4 + 1, sqrt(2));
  → (x^2 + sqrt(2)*x + 1) * (x^2 - sqrt(2)*x + 1)

algfactor(x^2 - 2, sqrt(2));
  → (x - sqrt(2)) * (x + sqrt(2))

algfactor(x^2 + 1, sqrt(2));
  → AlgFactor(x^2 + 1, sqrt(2))   [irreducible — returned unevaluated]
```

## Install

```bash
pip install coding-adventures-cas-algebraic
```

Or, for development:

```bash
uv pip install -e ../symbolic-ir -e ../cas-factor -e ".[dev]"
```

## Running tests

```bash
python -m pytest tests/ -v
```

## Coverage

Target: 80%+ line coverage.  Run with:

```bash
python -m pytest --cov=cas_algebraic --cov-report=term-missing
```
