# cas-fourier

Symbolic Fourier transform and inverse Fourier transform for the MACSYMA symbolic computation system.

## Overview

`cas-fourier` provides table-driven Fourier transforms for the `symbolic-vm` pipeline. It sits beside `cas-laplace` as part of the Group D transform packages in the MACSYMA CAS stack.

## Convention

Physics/engineering angular-frequency convention:

```
F(ω) = ∫_{-∞}^{+∞} f(t) · e^{-iωt} dt         (forward)

f(t) = (1/2π) ∫_{-∞}^{+∞} F(ω) · e^{+iωt} dω   (inverse)
```

The factor `1/(2π)` lives on the inverse, which means:
- `fourier(1) = 2π·δ(ω)` (all energy at ω=0)
- `ifourier(1) = δ(t)` (identity for the inverse)

## Forward transform table

| f(t) | F(ω) |
|------|------|
| `δ(t)` | `1` |
| `1` | `2π·δ(ω)` |
| `exp(-a·t)` (causal) | `1/(a + i·ω)` |
| `exp(i·a·t)` | `2π·δ(ω - a)` |
| `sin(ω₀·t)` | `i·π·(δ(ω+ω₀) - δ(ω-ω₀))` |
| `cos(ω₀·t)` | `π·(δ(ω-ω₀) + δ(ω+ω₀))` |
| `exp(-a·t²)` | `√(π/a) · exp(-ω²/(4a))` |
| `t·exp(-a·t)` (causal) | `1/(a + i·ω)²` |

Linearity rules are applied before the table:
- `fourier(c·f) = c·fourier(f)` for constant `c`
- `fourier(f + g) = fourier(f) + fourier(g)`

## Usage

```python
from symbolic_ir import IRSymbol, IRApply, IRInteger, EXP, NEG, MUL
from cas_fourier import fourier_transform, ifourier_transform

t = IRSymbol("t")
omega = IRSymbol("omega")

# δ(t) → 1
delta_t = IRApply(IRSymbol("DiracDelta"), (t,))
F = fourier_transform(delta_t, t, omega)
# F == IRInteger(1)

# exp(-2t) → 1/(2 + i·ω)
f = IRApply(EXP, (IRApply(NEG, (IRApply(MUL, (IRInteger(2), t)),)),))
F = fourier_transform(f, t, omega)

# Round-trip: ifourier(fourier(δ(t))) = δ(t)
recovered = ifourier_transform(F, omega, t)
```

## VM integration

Wire `cas-fourier` into the symbolic VM by calling `build_fourier_handler_table()` in `cas_handlers.py`:

```python
from cas_fourier import build_fourier_handler_table as _build_fourier
# In build_cas_handler_table():
    **_build_fourier(),
```

In MACSYMA sessions, the functions are available as `fourier(f, t, omega)` and `ifourier(F, omega, t)`.

## Architecture

```
cas_fourier/
  heads.py     — FOURIER and IFOURIER IR head symbols
  table.py     — Forward transform table + fourier_transform()
  inverse.py   — Inverse transform table + ifourier_transform()
  handlers.py  — VM handler functions + build_fourier_handler_table()
```

## Dependencies

- `coding-adventures-symbolic-ir >= 0.7.3` — IR node types and standard heads
- `coding-adventures-cas-laplace >= 0.1.0` — DiracDelta, UnitStep heads

## Layer position

This package is Layer D (transform packages) alongside `cas-laplace`. It depends on `symbolic-ir` for IR primitives and on `cas-laplace` for `DiracDelta` and `UnitStep` (which appear in its output).
