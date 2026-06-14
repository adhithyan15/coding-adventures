"""Inverse Laplace transform: inverse_laplace(F, s, t) → IRNode.

This is a thin wrapper that calls the inverse_table module's main
``inverse_laplace`` function. Keeping the public API here lets callers
do ``from cas_laplace.ilt import inverse_laplace`` without knowing which
submodule contains the implementation.

Usage
-----
::

    from symbolic_ir import IRSymbol
    from cas_laplace.ilt import inverse_laplace

    s = IRSymbol("s")
    t = IRSymbol("t")

    # L⁻¹{1/s} = UnitStep(t)
    result = inverse_laplace(Div(1, s), s, t)

    # L⁻¹{1/(s-2)} = exp(2t)
    result = inverse_laplace(Div(1, Sub(s, 2)), s, t)
"""

from __future__ import annotations

from cas_laplace.inverse_table import inverse_laplace

__all__ = ["inverse_laplace"]
