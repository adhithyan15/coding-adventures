"""VM handlers for complex-number IR heads.

All handlers follow the ``(VM, IRApply) -> IRNode`` signature contract:
if the input doesn't match expectations, return ``expr`` unchanged
(graceful fall-through / unevaluated).
"""
from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_complex.constants import IMAGINARY_UNIT, _SQRT, make_add, make_mul, make_pow
from cas_complex.normalize import contains_imaginary, normalize_complex, split_rect
from cas_complex.parts import conjugate, im_part, re_part
from cas_complex.polar import arg, polar_form, rect_form
from cas_complex.power import reduce_imaginary_power

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

# Local Handler type to avoid circular dependency on symbolic_vm at runtime.
Handler = Callable[["VM", IRApply], IRNode]

_TRUE = IRSymbol("True")
_FALSE = IRSymbol("False")


# ---------------------------------------------------------------------------
# ImaginaryUnit power reduction
# ---------------------------------------------------------------------------


def imaginary_power_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Pow(ImaginaryUnit, n)`` → one of ``{1, i, -1, -i}``.

    Only fires when the base is exactly ``ImaginaryUnit`` and the
    exponent is an ``IRInteger``.  All other ``Pow`` expressions are
    left for the normal VM arithmetic handler.
    """
    if len(expr.args) != 2:
        return expr
    base, exp = expr.args
    if not (isinstance(base, IRSymbol) and base.name == "ImaginaryUnit"):
        return expr
    if not isinstance(exp, IRInteger):
        return expr
    return vm.eval(reduce_imaginary_power(exp.value))


# ---------------------------------------------------------------------------
# Re / Im
# ---------------------------------------------------------------------------


def re_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Re(z)`` → real part of ``z``."""
    if len(expr.args) != 1:
        return expr
    return re_part(expr.args[0])


def im_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Im(z)`` → imaginary coefficient of ``z``."""
    if len(expr.args) != 1:
        return expr
    return im_part(expr.args[0])


# ---------------------------------------------------------------------------
# Conjugate
# ---------------------------------------------------------------------------


def conjugate_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Conjugate(z)`` → ``Re(z) - Im(z) * ImaginaryUnit``."""
    if len(expr.args) != 1:
        return expr
    return conjugate(expr.args[0])


# ---------------------------------------------------------------------------
# Abs (extended to complex inputs)
# ---------------------------------------------------------------------------


def abs_complex_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Abs(z)`` → ``√(a² + b²)`` when ``z`` contains ``ImaginaryUnit``.

    Falls through for real inputs (let the existing numeric Abs handler
    handle those).  This handler is registered under a private key and
    invoked from the main ``Abs`` dispatch below.
    """
    if len(expr.args) != 1:
        return expr
    z = expr.args[0]
    if not contains_imaginary(z):
        return expr  # fall through to real Abs

    real, imag = split_rect(normalize_complex(z))
    # sqrt(a^2 + b^2)
    sq_sum: IRNode = IRApply(
        IRSymbol("Add"),
        (
            IRApply(IRSymbol("Pow"), (real, IRInteger(2))),
            IRApply(IRSymbol("Pow"), (imag, IRInteger(2))),
        ),
    )
    return vm.eval(IRApply(_SQRT, (sq_sum,)))


# ---------------------------------------------------------------------------
# Arg
# ---------------------------------------------------------------------------


def arg_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Arg(z)`` → ``Atan2(Im(z), Re(z))``."""
    if len(expr.args) != 1:
        return expr
    return arg(expr.args[0])


# ---------------------------------------------------------------------------
# RectForm / PolarForm
# ---------------------------------------------------------------------------


def rect_form_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``RectForm(z)`` → ``a + b * ImaginaryUnit``."""
    if len(expr.args) != 1:
        return expr
    return rect_form(expr.args[0])


def polar_form_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``PolarForm(z)`` → ``r * Exp(ImaginaryUnit * theta)``."""
    if len(expr.args) != 1:
        return expr
    return polar_form(expr.args[0])


# ---------------------------------------------------------------------------
# Handler table
# ---------------------------------------------------------------------------


def build_complex_handler_table() -> dict[str, Handler]:
    """Return the handler table for complex-number IR heads.

    Intended to be merged into ``SymbolicBackend._handlers`` at
    construction time.  Two special cases:

    - ``"Pow"`` is NOT overridden here; the imaginary-unit power
      reduction is hooked in separately via
      :data:`IMAGINARY_POWER_HOOK`.  The main VM ``Pow`` handler should
      call :func:`imaginary_power_handler` when it detects
      ``ImaginaryUnit`` as the base.

    - ``"Abs"`` is extended: if the input is complex, delegate to
      :func:`abs_complex_handler`; otherwise fall through to the
      existing real Abs handler.
    """
    return {
        "Re": re_handler,
        "Im": im_handler,
        "Conjugate": conjugate_handler,
        "AbsComplex": abs_complex_handler,   # separate key; dispatched by AbsDispatch
        "Arg": arg_handler,
        "RectForm": rect_form_handler,
        "PolarForm": polar_form_handler,
        # ImaginaryUnit power reduction is wired separately.
        "_ImaginaryPow": imaginary_power_handler,
    }


# Expose the imaginary-power hook so SymbolicBackend can install it.
IMAGINARY_POWER_HOOK = imaginary_power_handler

#: Pre-bound constant exported for backend integration.
IMAGINARY_UNIT = IMAGINARY_UNIT
