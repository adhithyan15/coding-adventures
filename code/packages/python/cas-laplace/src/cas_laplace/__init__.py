"""cas-laplace — Laplace transform and inverse Laplace transform.

This package provides:

- ``laplace_transform(f, t, s)`` — forward Laplace transform via table lookup
- ``inverse_laplace(F, s, t)``   — inverse Laplace transform via partial fractions
- ``LAPLACE``, ``ILT``, ``DIRAC_DELTA``, ``UNIT_STEP`` — IR head symbols
- ``build_laplace_handler_table()`` — VM handler table for the symbolic VM

Quick start::

    from symbolic_ir import IRSymbol, IRApply, DIV, IRInteger, SIN, MUL
    from cas_laplace import laplace_transform, inverse_laplace
    from cas_laplace.heads import DIRAC_DELTA, UNIT_STEP

    t = IRSymbol("t")
    s = IRSymbol("s")

    # Forward: L{sin(2t)} = 2/(s^2 + 4)
    f = IRApply(SIN, (IRApply(MUL, (IRInteger(2), t)),))
    F = laplace_transform(f, t, s)

    # Inverse: L^{-1}{1/(s-3)} = exp(3t)
    from symbolic_ir import SUB
    F2 = IRApply(DIV, (IRInteger(1), IRApply(SUB, (s, IRInteger(3)))))
    f2 = inverse_laplace(F2, s, t)
"""

from __future__ import annotations

from cas_laplace.handlers import build_laplace_handler_table
from cas_laplace.heads import DIRAC_DELTA, ILT, LAPLACE, UNIT_STEP
from cas_laplace.ilt import inverse_laplace
from cas_laplace.laplace import laplace_transform

__all__ = [
    "laplace_transform",
    "inverse_laplace",
    "build_laplace_handler_table",
    "LAPLACE",
    "ILT",
    "DIRAC_DELTA",
    "UNIT_STEP",
]
