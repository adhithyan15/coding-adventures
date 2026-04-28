"""VM handlers for the Laplace transform operations.

These handlers integrate ``cas-laplace`` into the symbolic VM. The VM
dispatches on IR head names; when it sees ``Laplace(f, t, s)``, it calls
``laplace_handler``, which in turn calls ``laplace_transform``.

Handler contract
----------------
Every handler must follow the graceful fall-through contract:
- If the arguments are wrong (wrong arity, wrong types), return ``expr``
  unchanged (the unevaluated form).
- If the transform succeeds, return the transformed IR.
- The result is always passed through ``vm.eval()`` so that any further
  simplification the VM can do (e.g. 1*x → x) is applied automatically.

DiracDelta and UnitStep handlers
---------------------------------
These are "evaluation" handlers for the special function heads. They apply
only when the argument is a concrete number (or zero):

- DiracDelta(0) → 1    (sifting property at t=0)
- UnitStep(0)   → 1/2  (Heaviside convention H(0) = 1/2)
- UnitStep(-3)  → 0    (step not yet reached)
- UnitStep(5)   → 1    (step already passed)
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRInteger, IRNode, IRRational, IRSymbol

from cas_laplace.ilt import inverse_laplace
from cas_laplace.laplace import laplace_transform

if TYPE_CHECKING:
    from symbolic_vm.vm import VM


def laplace_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Laplace(f, t, s)`` — symbolic Laplace transform.

    Computes L{f(t)} with respect to the time variable ``t``, returning
    the result as an expression in the complex frequency variable ``s``.

    Falls through to the unevaluated form if:
    - Wrong arity (not exactly 3 arguments).
    - ``t`` or ``s`` is not an ``IRSymbol``.
    - No pattern in the transform table matches ``f``.
    """
    if len(expr.args) != 3:
        return expr
    f, t, s = expr.args
    if not isinstance(t, IRSymbol) or not isinstance(s, IRSymbol):
        return expr
    result = laplace_transform(f, t, s)
    return vm.eval(result)


def ilt_handler(vm: VM, expr: IRApply) -> IRNode:
    """``ILT(F, s, t)`` — inverse Laplace transform.

    Computes L⁻¹{F(s)} with respect to the complex frequency variable ``s``,
    returning the result as an expression in the time variable ``t``.

    Falls through to the unevaluated form if:
    - Wrong arity (not exactly 3 arguments).
    - ``s`` or ``t`` is not an ``IRSymbol``.
    - The function cannot be inverted by the current partial-fraction engine.
    """
    if len(expr.args) != 3:
        return expr
    F, s, t = expr.args
    if not isinstance(s, IRSymbol) or not isinstance(t, IRSymbol):
        return expr
    result = inverse_laplace(F, s, t)
    return vm.eval(result)


def dirac_delta_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``DiracDelta(x)`` — evaluate at a known numeric argument.

    Sifting property: the Dirac delta is "infinite" at zero in the
    distributional sense; for the purposes of symbolic evaluation,
    DiracDelta(0) evaluates to 1 (the integral ∫δ(t)dt = 1).

    For symbolic arguments, the expression remains unevaluated — we cannot
    reduce DiracDelta(x) when x is a free variable.
    """
    if len(expr.args) == 1:
        arg = expr.args[0]
        if isinstance(arg, IRInteger) and arg.value == 0:
            return IRInteger(1)
    return expr


def unit_step_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``UnitStep(x)`` — Heaviside step function evaluation.

    The Heaviside step function is defined as:
    - H(x) = 0 for x < 0  (signal has not arrived)
    - H(0) = 1/2           (convention: midpoint at the discontinuity)
    - H(x) = 1 for x > 0  (signal is present)

    Only evaluates for concrete ``IRInteger`` arguments. Symbolic arguments
    remain unevaluated.
    """
    if len(expr.args) == 1:
        arg = expr.args[0]
        if isinstance(arg, IRInteger):
            if arg.value < 0:
                return IRInteger(0)
            if arg.value > 0:
                return IRInteger(1)
            return IRRational(1, 2)  # H(0) = 1/2 convention
    return expr


def build_laplace_handler_table() -> dict[str, object]:
    """Return the handler table for the Laplace transform operations.

    Keys are canonical IR head names; values are handler callables
    with signature ``(VM, IRApply) -> IRNode``.
    """
    return {
        "Laplace": laplace_handler,
        "ILT": ilt_handler,
        "DiracDelta": dirac_delta_handler,
        "UnitStep": unit_step_handler,
    }
