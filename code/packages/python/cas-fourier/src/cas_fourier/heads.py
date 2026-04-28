"""IR head symbols for the Fourier transform package.

These two symbols are the canonical IR heads for the forward and inverse
Fourier transform operations. They are registered in:

- ``symbolic_ir.nodes``      — as module-level singletons (FOURIER, IFOURIER)
- ``symbolic_vm.cas_handlers`` — dispatch to fourier_handler / ifourier_handler
- ``macsyma_runtime.name_table`` — map ``fourier`` / ``ifourier`` to these heads

Design note
-----------
We keep the symbols as plain ``IRSymbol`` instances (not dataclass
subclasses) so that equality checks against the ones exported from
``symbolic_ir`` compare by value (name string).

Usage::

    from cas_fourier.heads import FOURIER, IFOURIER
    from symbolic_ir import IRApply, IRSymbol

    t = IRSymbol("t")
    omega = IRSymbol("omega")
    f = IRSymbol("f")

    # Build F(ω) = fourier(f, t, ω)  as an unevaluated IR node
    expr = IRApply(FOURIER, (f, t, omega))
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

# The forward Fourier transform head.
# Calling convention: Fourier(f, t, ω)
#   f     — the time-domain expression
#   t     — the integration variable (IRSymbol)
#   ω     — the frequency variable (IRSymbol)
FOURIER: IRSymbol = IRSymbol("Fourier")

# The inverse Fourier transform head.
# Calling convention: IFourier(F, ω, t)
#   F     — the frequency-domain expression
#   ω     — the frequency variable (IRSymbol)
#   t     — the time variable (IRSymbol)
IFOURIER: IRSymbol = IRSymbol("IFourier")
