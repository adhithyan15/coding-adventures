"""cas-ode — symbolic ODE solving for the MACSYMA CAS system.

This package provides:

- :func:`solve_ode` — pure-function ODE dispatcher (separable, linear,
  Bernoulli-flavoured linear, 2nd-order constant-coefficient)
- :func:`build_ode_handler_table` — VM handler table for the symbolic VM
- ``ODE2``, ``C1``, ``C2``, ``C_CONST`` — IR head symbols (re-exported)

Quick start::

    from symbolic_ir import IRSymbol, IRApply, IRInteger
    from symbolic_ir.nodes import D, ODE2, C_CONST, C1, C2
    from cas_ode import solve_ode, build_ode_handler_table

    x = IRSymbol("x")
    y = IRSymbol("y")

    # Build the ODE: y' - 2*y = 0  (first-order linear)
    y_prime = IRApply(D, (y, x))
    from symbolic_ir import SUB, MUL
    expr = IRApply(SUB, (y_prime, IRApply(MUL, (IRInteger(2), y))))

    # Then either call solve_ode(expr, y, x, vm) with a live VM,
    # or use the ODE2 handler through the VM:
    #   vm.eval(IRApply(ODE2, (expr, y, x)))

Architecture
------------
The four-step integration pattern:

1. ``cas_ode.ode`` — pure solvers operating on IRNode trees.
2. ``cas_ode.handlers.build_ode_handler_table()`` — wraps solvers as VM
   handlers keyed by IR head name.
3. ``symbolic_vm/cas_handlers.py`` — receives ``handlers.update(...)``
   from ``build_ode_handler_table``.
4. ``macsyma_runtime/name_table.py`` — maps ``"ode2"`` to ``ODE2``.
"""

from __future__ import annotations

from symbolic_ir.nodes import C1, C2, C_CONST, ODE2

from cas_ode.handlers import build_ode_handler_table
from cas_ode.ode import solve_ode

__all__ = [
    "solve_ode",
    "build_ode_handler_table",
    "ODE2",
    "C1",
    "C2",
    "C_CONST",
]
