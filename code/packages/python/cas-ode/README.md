# cas-ode

Symbolic ODE solver for the MACSYMA CAS system (part of the coding-adventures
computer algebra stack).

## What it solves

| ODE Type | Form | Method |
|----------|------|--------|
| First-order linear | `y' + P(x)·y = Q(x)` | Integrating factor |
| Separable | `y' = f(x)·g(y)` | Separation of variables |
| 2nd-order const-coeff | `a·y'' + b·y' + c·y = 0` | Characteristic equation |

## MACSYMA surface syntax

```
ode2(y' - 2*y, y, x)        -> y = %c * exp(2*x)
ode2(y'' + y, y, x)         -> y = %c1*cos(x) + %c2*sin(x)
ode2(y'' - 2*y' + y, y, x)  -> y = (%c1 + %c2*x)*exp(x)
```

## Integration constants

- First-order: `%c`
- Second-order: `%c1`, `%c2`

## Usage

```python
from symbolic_ir import IRSymbol, IRApply, IRInteger, SUB, MUL
from symbolic_ir.nodes import D, ODE2
from symbolic_vm import VM, SymbolicBackend
from cas_ode import build_ode_handler_table

x = IRSymbol("x")
y = IRSymbol("y")

# Set up VM with ODE handler
backend = SymbolicBackend()
backend._handlers.update(build_ode_handler_table())
vm = VM(backend)

# Solve y' - 2*y = 0
y_prime = IRApply(D, (y, x))
expr = IRApply(SUB, (y_prime, IRApply(MUL, (IRInteger(2), y))))
result = vm.eval(IRApply(ODE2, (expr, y, x)))
# -> Equal(y, Mul(%c, Exp(Mul(2, x))))
```

## Architecture

Follows the four-step CAS integration pattern:

1. `cas_ode/ode.py` — pure IR-to-IR solver functions
2. `cas_ode/handlers.py` — VM handler wrapping + `build_ode_handler_table()`
3. Wired into `symbolic_vm/cas_handlers.py`
4. Name registered in `macsyma_runtime/name_table.py`
