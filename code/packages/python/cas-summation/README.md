# cas-summation

Symbolic summation library for the coding-adventures CAS stack.

Evaluates `sum(f, k, a, b)` and `product(f, k, a, b)` to closed forms:

| Input | Output |
|-------|--------|
| `sum(k^2, k, 1, n)` | `n*(n+1)*(2*n+1)/6` |
| `sum(1/2^k, k, 0, inf)` | `2` |
| `sum(1/k^2, k, 1, inf)` | `%pi^2/6` |
| `sum(1/k!, k, 0, inf)` | `%e` |
| `sum(x^k/k!, k, 0, inf)` | `exp(x)` |
| `product(k, k, 1, n)` | `GammaFunc(n+1)` |

## Usage

```python
from symbolic_ir import IRSymbol, IRInteger
from cas_summation import evaluate_sum

n = IRSymbol("n")
k = IRSymbol("k")

# sum(k^2, k, 1, n)
from symbolic_ir import IRApply, POW
f = IRApply(POW, (k, IRInteger(2)))
result = evaluate_sum(f, k, IRInteger(1), n, vm)
```

## Supported families

**Sums:**
- Constant (independent of k)
- Geometric series (finite and infinite)
- Powers k^m for m = 0…5 via Faulhaber's formula
- Classic infinite series: Basel, Leibniz, Taylor for e and exp(x)
- Numeric direct computation for small concrete ranges

**Products:**
- Constant factor
- Factorial: product(k, k, 1, n) = n! = GammaFunc(n+1)
- Scaled factorial
- Numeric direct computation

## Stack position

```
macsyma-runtime
    └── symbolic-vm  ← routes SUM/PRODUCT to this package
            └── cas-summation  ← you are here
                    └── symbolic-ir  (SUM, PRODUCT, GAMMA_FUNC heads)
```
