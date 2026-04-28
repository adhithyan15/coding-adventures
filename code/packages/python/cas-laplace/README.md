# cas-laplace

Laplace transform and inverse Laplace transform for the MACSYMA symbolic computation system.

## Overview

`cas-laplace` implements:

- `laplace_transform(f, t, s)` — symbolic Laplace transform via table lookup + linearity
- `inverse_laplace(F, s, t)` — inverse Laplace transform via partial fractions + inverse table
- `LAPLACE`, `ILT`, `DIRAC_DELTA`, `UNIT_STEP` — canonical IR head symbols
- `build_laplace_handler_table()` — VM handler table for integration with `symbolic-vm`

## Mathematical background

The Laplace transform converts a time-domain function f(t) into a complex-frequency-domain function F(s):

```
L{f(t)} = F(s) = ∫₀^∞ f(t) e^{-st} dt
```

It is the primary tool for solving ordinary differential equations with initial conditions, and for analyzing linear time-invariant (LTI) systems in control theory and signal processing.

## Usage

```python
from symbolic_ir import IRSymbol, IRApply, MUL, IRInteger, SIN, EXP, DIV, SUB
from cas_laplace import laplace_transform, inverse_laplace

t = IRSymbol("t")
s = IRSymbol("s")

# Forward: L{sin(2t)} = 2/(s^2 + 4)
f = IRApply(SIN, (IRApply(MUL, (IRInteger(2), t)),))
F = laplace_transform(f, t, s)

# Forward: L{exp(3t)} = 1/(s-3)
f2 = IRApply(EXP, (IRApply(MUL, (IRInteger(3), t)),))
F2 = laplace_transform(f2, t, s)

# Inverse: L^{-1}{1/(s-3)} = exp(3*t)
F3 = IRApply(DIV, (IRInteger(1), IRApply(SUB, (s, IRInteger(3)))))
f3 = inverse_laplace(F3, s, t)

# Linearity: L{3*sin(t) + cos(t)}
from symbolic_ir import ADD, COS
f4 = IRApply(ADD, (IRApply(MUL, (IRInteger(3), IRApply(SIN, (t,)))), IRApply(COS, (t,))))
F4 = laplace_transform(f4, t, s)
```

## Transform table

| f(t) | L{f}(s) |
|------|---------|
| 1 | 1/s |
| t^n | n!/s^{n+1} |
| exp(at) | 1/(s-a) |
| sin(ωt) | ω/(s²+ω²) |
| cos(ωt) | s/(s²+ω²) |
| exp(at)·sin(ωt) | ω/((s-a)²+ω²) |
| exp(at)·cos(ωt) | (s-a)/((s-a)²+ω²) |
| t·exp(at) | 1/(s-a)² |
| t^n·exp(at) | n!/(s-a)^{n+1} |
| sinh(at) | a/(s²-a²) |
| cosh(at) | s/(s²-a²) |
| t·sin(ωt) | 2ωs/(s²+ω²)² |
| t·cos(ωt) | (s²-ω²)/(s²+ω²)² |
| DiracDelta(t) | 1 |
| UnitStep(t) | 1/s |

## VM integration

Wire this package into `symbolic-vm` by calling `build_laplace_handler_table()`:

```python
# In symbolic_vm/cas_handlers.py
from cas_laplace import build_laplace_handler_table as _build_laplace

def build_cas_handler_table():
    return {
        ...
        **_build_laplace(),
    }
```

## Special function heads

`DIRAC_DELTA` and `UNIT_STEP` are canonical IR heads defined in this package.
They are also intended for use by the future `cas-fourier` package.

- `DiracDelta(t)` — Dirac delta distribution δ(t)
- `UnitStep(t)` — Heaviside step function u(t)

In MACSYMA syntax, these are accessible as `delta(t)` and `hstep(t)`.
