"""VM handlers for the ``ODE2`` operation.

This module integrates ``cas-ode`` into the symbolic VM. The VM
dispatches on IR head names; when it sees ``ODE2(eqn, y, x)`` it calls
:func:`ode2_handler`, which classifies the ODE and delegates to the
appropriate solver in :mod:`cas_ode.ode`.

Handler contract
----------------
Every handler must follow the graceful fall-through contract:
- If the arguments are wrong (wrong arity, wrong types), return ``expr``
  unchanged (the unevaluated form).
- If the solver succeeds, return ``Equal(y, solution)``.
- If the solver cannot handle the ODE, return ``expr`` unchanged
  (the unevaluated ``ODE2(…)``).  The user sees the operation they
  typed, which is the standard CAS behaviour for "I don't know".

ODE input normalisation
-----------------------
The ``eqn`` argument may be:

1. A raw expression (treated as ``expr = 0``).
2. An ``Equal(lhs, rhs)`` node — rearranged to ``lhs - rhs = 0``.

This matches MACSYMA's ``ode2(eqn, y, x)`` surface syntax, where the
user can write either ``ode2(y' - 2*y, y, x)`` (expression form) or
``ode2(y' = 2*y, y, x)`` (equation form).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import (
    EQUAL,
    SUB,
    IRApply,
    IRNode,
    IRSymbol,
)

from cas_ode.ode import solve_ode

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


def ode2_handler(vm: VM, expr: IRApply) -> IRNode:
    """``ODE2(eqn, y, x)`` — symbolic ODE solver.

    Attempts to solve the given ODE for ``y`` as a function of ``x``.
    Supported ODE types (in dispatch order):

    1. Second-order constant-coefficient homogeneous:
       ``a·y'' + b·y' + c·y = 0`` → characteristic equation approach.

    2. First-order linear:
       ``y' + P(x)·y = Q(x)`` → integrating factor ``μ = exp(∫P dx)``.

    3. Separable:
       ``y' = f(x)·g(y)`` → separation of variables.

    Parameters
    ----------
    vm:
        The live VM instance — used for evaluating ``Integrate(f, x)``
        during the integrating-factor computation.
    expr:
        The ``IRApply(ODE2, (eqn, y, x))`` call node.

    Returns
    -------
    ``Equal(y, solution)`` on success, or ``expr`` unchanged on failure.

    Examples
    --------
    First-order linear (decaying)::

        ode2(y' + 2*y, y, x)  →  Equal(y, Mul(%c, Exp(Mul(-2, x))))

    Second-order (complex roots)::

        ode2(y'' + y, y, x)   →  Equal(y, Mul(Exp(0), Add(Mul(%c1, Cos(x)),
                                                            Mul(%c2, Sin(x)))))
    """
    if len(expr.args) != 3:
        return expr

    eqn, y_sym, x_sym = expr.args

    if not isinstance(y_sym, IRSymbol) or not isinstance(x_sym, IRSymbol):
        return expr

    # ---- Normalise the equation to zero form --------------------------------
    # If the user wrote Equal(lhs, rhs), move rhs to the left.
    zero_form: IRNode
    if (
        isinstance(eqn, IRApply)
        and isinstance(eqn.head, IRSymbol)
        and eqn.head == EQUAL
        and len(eqn.args) == 2
    ):
        lhs, rhs = eqn.args
        zero_form = IRApply(SUB, (lhs, rhs))
    else:
        zero_form = eqn

    # ---- Delegate to the solver ---------------------------------------------
    result = solve_ode(zero_form, y_sym, x_sym, vm)
    if result is None:
        return expr  # Unevaluated — fall through

    return result


def build_ode_handler_table() -> dict[str, object]:
    """Return the handler table for ODE solving.

    Keys are canonical IR head names; values are handler callables
    with signature ``(VM, IRApply) -> IRNode``.

    Wire this into the symbolic VM via::

        from cas_ode import build_ode_handler_table
        handlers.update(build_ode_handler_table())
    """
    return {
        "ODE2": ode2_handler,
    }
