"""Handler table for the MNewton IR head.

This module bridges the ``symbolic_vm`` evaluation pipeline with the
pure ``mnewton_solve`` function in ``newton.py``. Keeping them separate
means the core algorithm has no dependency on the VM, making it easy to
test in isolation and reuse in other contexts.

The handler follows the standard VM contract:
- Accept ``(vm, expr: IRApply) -> IRNode``.
- Return ``expr`` unchanged (unevaluated) for any malformed input.
- Return ``IRFloat(root)`` on success.

Wire-up is deferred to ``build_mnewton_handler_table()`` which returns
the dict expected by ``SymbolicBackend``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRFloat, IRInteger, IRNode, IRRational, IRSymbol

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

from cas_mnewton.newton import MNewtonError, mnewton_solve

# The canonical head symbol for Newton's method in the IR.
# Using a module-level constant ensures every part of the system refers
# to the same object (identity comparison is then O(1)).
MNEWTON = IRSymbol("MNewton")


def mnewton_handler(vm: VM, expr: IRApply) -> IRNode:
    """``MNewton(f, x, x0)`` or ``MNewton(f, x, x0, tol)`` — Newton root finder.

    Finds a root of f near x0 numerically by iterating:

        x_{n+1} = x_n - f(x_n) / f'(x_n)

    where f' is computed symbolically *once* and then evaluated on each
    iteration by substituting x_n.

    Arguments
    ---------
    f   : the function expression (symbolic, contains x).
    x   : the variable symbol (must be an ``IRSymbol``).
    x0  : the initial guess (must be a numeric IR literal).
    tol : optional tolerance (default 1e-10). Must be a numeric literal.

    Returns
    -------
    ``IRFloat(root)`` on convergence; the original expression on failure.

    Failure modes
    -------------
    - Wrong arity (not 3 or 4 args) → unevaluated.
    - ``x`` is not an ``IRSymbol``  → unevaluated.
    - ``tol`` arg is not numeric    → unevaluated.
    - x0 is not numeric             → unevaluated (handled in newton.py).
    - Derivative is zero at x0      → unevaluated (MNewtonError caught).
    """
    # Validate arity first — the VM contract says malformed calls
    # fall through to unevaluated, never raise.
    if len(expr.args) not in (3, 4):
        return expr

    # Extract positional arguments.
    if len(expr.args) == 3:
        f_ir, x_sym, x0_ir = expr.args
        tol = 1e-10  # default tolerance
    else:
        f_ir, x_sym, x0_ir, tol_ir = expr.args
        # Convert the tolerance argument to a Python float.
        # We accept all three numeric IR types.
        if isinstance(tol_ir, IRFloat):
            tol = tol_ir.value
        elif isinstance(tol_ir, IRInteger):
            tol = float(tol_ir.value)
        elif isinstance(tol_ir, IRRational):
            tol = tol_ir.numer / tol_ir.denom
        else:
            # Symbolic tolerance — cannot proceed.
            return expr

    # x must be a plain symbol — Newton's method operates in one variable.
    if not isinstance(x_sym, IRSymbol):
        return expr

    # Import here to avoid a circular dependency at module-load time.
    # symbolic_vm imports cas_mnewton (to register the handler), so we
    # must not import symbolic_vm at the top of this file.
    from symbolic_vm.derivative import _diff

    try:
        return mnewton_solve(f_ir, x_sym, x0_ir, vm.eval, _diff, tol=tol)
    except MNewtonError:
        # Derivative vanished — return the original unevaluated expression.
        return expr


def build_mnewton_handler_table() -> dict[str, object]:
    """Return the handler dict for wiring MNewton into SymbolicBackend.

    Usage in ``cas_handlers.py``::

        from cas_mnewton import build_mnewton_handler_table as _build_mnewton
        ...
        return {
            ...
            **_build_mnewton(),
        }
    """
    return {"MNewton": mnewton_handler}
