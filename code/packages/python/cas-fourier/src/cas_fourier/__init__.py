"""cas-fourier — Symbolic Fourier transform and inverse for the symbolic VM.

This package provides:

- ``fourier_transform(f, t, ω)``  — forward Fourier transform via table
- ``ifourier_transform(F, ω, t)`` — inverse Fourier transform via table
- ``FOURIER``, ``IFOURIER``       — IR head symbols
- ``build_fourier_handler_table()`` — VM handler table fragment

Convention
----------
Physics/engineering angular-frequency form::

    F(ω) = ∫_{-∞}^{+∞} f(t) e^{-iωt} dt

    f(t) = (1/2π) ∫_{-∞}^{+∞} F(ω) e^{+iωt} dω

Quick start::

    from symbolic_ir import IRSymbol, IRApply, IRInteger, MUL
    from cas_fourier import fourier_transform, ifourier_transform, FOURIER
    from cas_fourier.heads import DIRAC_DELTA  # re-exported convenience

    t = IRSymbol("t")
    omega = IRSymbol("omega")

    # δ(t) → 1
    from symbolic_ir import IRApply, IRSymbol
    delta_t = IRApply(IRSymbol("DiracDelta"), (t,))
    result = fourier_transform(delta_t, t, omega)
    # result == IRInteger(1)

    # exp(-2t) → 1/(2 + i·ω)
    from symbolic_ir import EXP, NEG
    f = IRApply(EXP, (IRApply(NEG, (IRApply(MUL, (IRInteger(2), t)),)),))
    F = fourier_transform(f, t, omega)
"""

from __future__ import annotations

from cas_fourier.handlers import build_fourier_handler_table
from cas_fourier.heads import FOURIER, IFOURIER
from cas_fourier.inverse import ifourier_transform
from cas_fourier.table import fourier_transform

__all__ = [
    "fourier_transform",
    "ifourier_transform",
    "build_fourier_handler_table",
    "FOURIER",
    "IFOURIER",
]
