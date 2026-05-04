# cas-mnewton

Newton's method numeric root finder for the MACSYMA symbolic VM.

## What it does

`cas-mnewton` implements the **Newton-Raphson method** for finding roots of symbolic expressions numerically. Given a function `f(x)` and an initial guess `x0`, it iterates:

    x_{n+1} = x_n - f(x_n) / f'(x_n)

until `|f(x_n)| < tol` (default `1e-10`). The derivative `f'(x)` is computed **symbolically once** before the loop using the VM's differentiation engine, then evaluated numerically on each iteration — combining the best of symbolic and numeric computation.

## How it fits in the stack

```
macsyma-runtime  ──►  symbolic-vm  ──►  cas-mnewton
                                         │
                                         ├── newton.py     (pure algorithm)
                                         └── handlers.py   (VM wiring)
```

`cas-mnewton` depends only on `symbolic-ir` and `cas-substitution`. The VM integration (`symbolic_vm.derivative._diff`) is imported lazily inside the handler to avoid circular imports.

## MACSYMA syntax

```
mnewton(x^2 - 2, x, 1.5)          → 1.4142135623730951
mnewton(x^3 - 8, x, 1.0)          → 2.0
mnewton(sin(x), x, 3.0)            → 3.141592653589793
mnewton(x^2 - 2, x, 1.5, 1e-6)    → root with looser tolerance
```

## Python API

```python
from cas_mnewton import mnewton_solve, MNewtonError
from symbolic_ir import IRApply, IRFloat, IRInteger, IRSymbol, SUB, POW

x = IRSymbol("x")
f = IRApply(SUB, (IRApply(POW, (x, IRInteger(2))), IRInteger(2)))  # x^2 - 2

# With a live VM:
from symbolic_vm import VM, SymbolicBackend
from symbolic_vm.derivative import _diff

vm = VM(SymbolicBackend())
result = mnewton_solve(f, x, IRFloat(1.5), vm.eval, _diff)
# result = IRFloat(1.4142135623730951)
```

## Edge cases

| Situation | Behaviour |
|---|---|
| `x0` is not numeric | Returns unevaluated |
| `f'(x0) = 0` | Returns unevaluated |
| var is not `IRSymbol` | Returns unevaluated |
| Exceeds `max_iter` (50) | Returns best approximation |

## Installation

```
pip install coding-adventures-cas-mnewton
```
