"""VM handlers for complex-number IR heads.

All handlers follow the ``(VM, IRApply) -> IRNode`` signature contract:
if the input doesn't match expectations, return ``expr`` unchanged
(graceful fall-through / unevaluated).
"""
from __future__ import annotations

import math as _math
from collections.abc import Callable
from typing import TYPE_CHECKING

from symbolic_ir import IRApply, IRFloat, IRInteger, IRNode, IRRational, IRSymbol

from cas_complex.constants import _SQRT, IMAGINARY_UNIT, make_mul
from cas_complex.normalize import contains_imaginary, normalize_complex, split_rect
from cas_complex.parts import conjugate, im_part, re_part
from cas_complex.polar import arg, polar_form, rect_form
from cas_complex.power import reduce_imaginary_power

if TYPE_CHECKING:
    from symbolic_vm.vm import VM

# Local Handler type to avoid circular dependency on symbolic_vm at runtime.
Handler = Callable[["VM", IRApply], IRNode]


# ---------------------------------------------------------------------------
# Local numeric helpers — avoids a runtime import of symbolic_vm.numeric
# (which would pull in symbolic_vm.__init__ → backends → cas_trig, creating
# a heavy transitive dependency that breaks isolated testing of this package).
# ---------------------------------------------------------------------------


def _to_number(node: IRNode) -> float | None:
    """Return the numeric value of a literal node, or ``None`` if symbolic.

    Handles ``IRInteger``, ``IRFloat``, and ``IRRational``.
    """
    if isinstance(node, IRInteger):
        return float(node.value)
    if isinstance(node, IRFloat):
        return node.value
    if isinstance(node, IRRational):
        return node.numer / node.denom
    return None


def _from_number(value: float) -> IRNode:
    """Wrap a Python float as an ``IRFloat`` (or ``IRInteger`` when exact)."""
    iv = int(value)
    if float(iv) == value:
        return IRInteger(iv)
    return IRFloat(value)


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


def arg_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Arg(z)`` → principal argument of ``z``.

    For complex inputs: ``Atan2(Im(z), Re(z))`` evaluated numerically.
    For purely real numeric inputs: ``atan2(0, x)`` → 0 or π.
    For symbolic real inputs: returned unevaluated.

    The handler must NOT call ``vm.eval()`` on a still-unevaluated
    ``Arg(...)`` node or it will recurse indefinitely.
    """
    if len(expr.args) != 1:
        return expr
    z = expr.args[0]
    result = arg(z)
    # arg() returns Atan2(im, re) when z is complex — evaluate it.
    if (
        isinstance(result, IRApply)
        and isinstance(result.head, IRSymbol)
        and result.head.name == "Atan2"
    ):
        return vm.eval(result)
    # arg() returned unevaluated Arg(x) — z is purely real.
    # Try numeric evaluation: Arg(x) = atan2(0, x).
    val = _to_number(z)
    if val is not None:
        return _from_number(_math.atan2(0.0, float(val)))
    return result  # symbolic real — leave unevaluated


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
# complex_mul_handler
# ---------------------------------------------------------------------------


def _is_zero_node(node: IRNode) -> bool:
    """Return True if ``node`` is a numeric zero.

    Handles ``IRInteger(0)``, ``IRRational(0/n)``, and ``IRFloat(0.0)``.
    The float case arises when the VM evaluates arithmetic sub-expressions
    like ``Sub(Mul(0, x), Mul(y, 0))`` and returns a floating-point zero.
    """
    if isinstance(node, IRInteger):
        return node.value == 0
    if isinstance(node, IRRational):
        return node.numer == 0
    if isinstance(node, IRFloat):
        return node.value == 0.0
    return False


def complex_mul_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Mul(a, b)`` where at least one arg contains ``ImaginaryUnit``.

    Expands ``(re_a + im_a*i) * (re_b + im_b*i)``
    → ``(re_a*re_b - im_a*im_b) + (re_a*im_b + im_a*re_b)*i``
    then normalizes.

    Falls through (returns ``expr``) when both imaginary parts resolve to
    zero after ``split_rect`` — this prevents infinite recursion on
    expressions like ``Mul(2, Pow(e, i*theta))`` where ``split_rect``
    cannot unwrap the ``Pow`` into a rectangular form.
    """
    if len(expr.args) != 2:
        return expr
    a, b = expr.args
    if not (contains_imaginary(a) or contains_imaginary(b)):
        return expr  # not complex — fall through

    re_a, im_a = split_rect(normalize_complex(a))
    re_b, im_b = split_rect(normalize_complex(b))

    # If neither operand has an explicit imaginary part after split_rect,
    # we cannot expand further and must fall through to avoid recursion.
    # This happens when one operand is a complex sub-expression that
    # split_rect cannot decompose (e.g. Pow(e, i*theta)).
    if _is_zero_node(im_a) and _is_zero_node(im_b):
        return expr

    _MUL = IRSymbol("Mul")
    _SUB = IRSymbol("Sub")

    real_part = vm.eval(IRApply(_SUB, (
        IRApply(_MUL, (re_a, re_b)),
        IRApply(_MUL, (im_a, im_b)),
    )))
    imag_part = vm.eval(IRApply(IRSymbol("Add"), (
        IRApply(_MUL, (re_a, im_b)),
        IRApply(_MUL, (im_a, re_b)),
    )))

    return normalize_complex(IRApply(IRSymbol("Add"), (
        real_part,
        make_mul(imag_part, IMAGINARY_UNIT),
    )))


# ---------------------------------------------------------------------------
# complex_div_handler — Div(z, w) where numerator or both are complex
# ---------------------------------------------------------------------------


def complex_div_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Div(z, c)`` or ``Div(z, w)`` when ``z`` contains ``ImaginaryUnit``.

    Two sub-cases:

    - **Real denominator** ``Div(a + b*i, c)`` → ``a/c + (b/c)*i``.
      This is the common case when dividing by a scalar (e.g. ``%i*%pi/2``).

    - **Complex denominator** ``Div(a + b*i, c + d*i)`` →
      ``((a*c + b*d) + (b*c - a*d)*i) / (c² + d²)``
      using the standard conjugate-multiply trick.

    Falls through for non-complex numerators (let the existing numeric
    Div handler handle those).
    """
    if len(expr.args) != 2:
        return expr
    numerator, denominator = expr.args
    if not contains_imaginary(numerator):
        return expr  # fall through — numerator is real

    re_n, im_n = split_rect(normalize_complex(numerator))
    _DIV = IRSymbol("Div")

    if not contains_imaginary(denominator):
        # Simple case: (a + b*i) / c  →  a/c + (b/c)*i
        real_part = vm.eval(IRApply(_DIV, (re_n, denominator)))
        imag_part = vm.eval(IRApply(_DIV, (im_n, denominator)))
        return normalize_complex(IRApply(IRSymbol("Add"), (
            real_part,
            make_mul(imag_part, IMAGINARY_UNIT),
        )))

    # Full complex division: (a + b*i) / (c + d*i)
    # = ((a*c + b*d) + (b*c - a*d)*i) / (c² + d²)
    re_d, im_d = split_rect(normalize_complex(denominator))
    _MUL = IRSymbol("Mul")
    _ADD = IRSymbol("Add")
    _SUB = IRSymbol("Sub")

    mag_sq = vm.eval(IRApply(_ADD, (
        IRApply(_MUL, (re_d, re_d)),
        IRApply(_MUL, (im_d, im_d)),
    )))
    real_num = vm.eval(IRApply(_ADD, (
        IRApply(_MUL, (re_n, re_d)),
        IRApply(_MUL, (im_n, im_d)),
    )))
    imag_num = vm.eval(IRApply(_SUB, (
        IRApply(_MUL, (im_n, re_d)),
        IRApply(_MUL, (re_n, im_d)),
    )))
    real_part = vm.eval(IRApply(_DIV, (real_num, mag_sq)))
    imag_part = vm.eval(IRApply(_DIV, (imag_num, mag_sq)))
    return normalize_complex(IRApply(IRSymbol("Add"), (
        real_part,
        make_mul(imag_part, IMAGINARY_UNIT),
    )))


# ---------------------------------------------------------------------------
# complex_pow_handler — Pow(z, n) where z is complex and n is a positive int
# ---------------------------------------------------------------------------


def complex_pow_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Pow(z, n)`` → expand when ``z`` contains ``ImaginaryUnit`` and ``n``
    is a small positive integer.

    Uses the recurrence ``z^n = z * z^(n-1)``, bottoming out at ``n=1``.
    Each recursive step goes through the VM so that ``complex_mul_handler``
    can reduce the intermediate complex products to rectangular form.

    Guards:
    - Falls through for ``n <= 0`` (negative/zero powers are not expanded).
    - Falls through for ``n > 16`` (large powers are left symbolic to avoid
      deep recursion in interactive sessions).
    - Falls through when the base is ``ImaginaryUnit`` itself (handled by
      the faster ``imaginary_power_handler``).
    """
    if len(expr.args) != 2:
        return expr
    base, exp = expr.args
    # ImaginaryUnit^n is handled by IMAGINARY_POWER_HOOK — don't touch it.
    if isinstance(base, IRSymbol) and base.name == "ImaginaryUnit":
        return expr
    if not contains_imaginary(base):
        return expr
    if not isinstance(exp, IRInteger):
        return expr
    n = exp.value
    if n <= 0 or n > 16:
        return expr  # leave very large or non-positive integer powers symbolic
    if n == 1:
        return base
    # z^n = z * z^(n-1)
    sub_pow = vm.eval(IRApply(IRSymbol("Pow"), (base, IRInteger(n - 1))))
    return vm.eval(IRApply(IRSymbol("Mul"), (base, sub_pow)))


# ---------------------------------------------------------------------------
# euler_pow_handler — Pow(b, i*theta) via Euler's formula
# ---------------------------------------------------------------------------


def euler_pow_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Pow(b, i*theta)`` → ``cos(ln(b)*theta) + i*sin(ln(b)*theta)``.

    Fires when the exponent is purely imaginary ``i*theta`` and the base
    is a positive real number.  Uses the identity::

        b^(i*theta) = e^(i*theta*ln(b)) = cos(theta*ln(b)) + i*sin(theta*ln(b))

    Falls through for:
    - Non-imaginary exponents (handled elsewhere).
    - ``ImaginaryUnit^n`` (handled by ``IMAGINARY_POWER_HOOK``).
    - Non-positive or non-numeric bases (can't take log).
    """
    if len(expr.args) != 2:
        return expr
    base, exp = expr.args
    # ImaginaryUnit^n is for IMAGINARY_POWER_HOOK, not here.
    if isinstance(base, IRSymbol) and base.name == "ImaginaryUnit":
        return expr
    # Fast path: Mul(i, theta) or Mul(theta, i) before normalization.
    theta = _extract_imaginary_exponent(exp)

    # Fallback: exponent is in normalized rect form 0 + theta*i.
    # This occurs after complex_mul_handler has already run on i*theta.
    if theta is None and contains_imaginary(exp):
        re_e, im_e = split_rect(exp)
        if _is_zero_node(re_e):
            theta = im_e

    if theta is None:
        return expr  # not a purely imaginary exponent

    b_val = _to_number(base)
    if b_val is None or float(b_val) <= 0:
        return expr  # need positive real base for log

    log_b = _from_number(_math.log(float(b_val)))
    # angle = ln(b) * theta
    angle = vm.eval(IRApply(IRSymbol("Mul"), (log_b, theta)))
    cos_a = vm.eval(IRApply(IRSymbol("Cos"), (angle,)))
    sin_a = vm.eval(IRApply(IRSymbol("Sin"), (angle,)))
    return normalize_complex(IRApply(IRSymbol("Add"), (
        cos_a,
        make_mul(sin_a, IMAGINARY_UNIT),
    )))


# ---------------------------------------------------------------------------
# exp_complex_handler (Euler's formula)
# ---------------------------------------------------------------------------


def _extract_imaginary_exponent(exp_arg: IRNode) -> IRNode | None:
    """Return ``theta`` if ``exp_arg`` is ``Mul(ImaginaryUnit, theta)``
    or ``Mul(theta, ImaginaryUnit)``, else ``None``."""
    if not (
        isinstance(exp_arg, IRApply)
        and isinstance(exp_arg.head, IRSymbol)
        and exp_arg.head.name == "Mul"
        and len(exp_arg.args) == 2
    ):
        return None
    a, b = exp_arg.args
    if isinstance(a, IRSymbol) and a.name == "ImaginaryUnit":
        return b
    if isinstance(b, IRSymbol) and b.name == "ImaginaryUnit":
        return a
    return None


def exp_complex_handler(vm: VM, expr: IRApply) -> IRNode:
    """``Exp(i*theta)`` → ``cos(theta) + i*sin(theta)`` (Euler's formula).

    Only fires when the exponent is purely imaginary (``i*theta``).
    For real exponents, falls through unchanged.
    For general ``Exp(a + i*b)``, returns ``Exp(a) * (cos(b) + i*sin(b))``.
    """
    if len(expr.args) != 1:
        return expr
    arg = expr.args[0]

    # Pure imaginary exponent: Exp(i*theta)
    theta = _extract_imaginary_exponent(arg)
    if theta is not None:
        cos_t = vm.eval(IRApply(IRSymbol("Cos"), (theta,)))
        sin_t = vm.eval(IRApply(IRSymbol("Sin"), (theta,)))
        return normalize_complex(IRApply(IRSymbol("Add"), (
            cos_t,
            make_mul(sin_t, IMAGINARY_UNIT),
        )))

    # General complex exponent: Exp(a + i*b)
    if contains_imaginary(arg):
        real_part, imag_part = split_rect(normalize_complex(arg))
        r = vm.eval(IRApply(IRSymbol("Exp"), (real_part,)))
        cos_b = vm.eval(IRApply(IRSymbol("Cos"), (imag_part,)))
        sin_b = vm.eval(IRApply(IRSymbol("Sin"), (imag_part,)))
        # r * (cos(b) + i*sin(b))
        rect = normalize_complex(IRApply(IRSymbol("Add"), (
            cos_b,
            make_mul(sin_b, IMAGINARY_UNIT),
        )))
        return vm.eval(IRApply(IRSymbol("Mul"), (r, rect)))

    return expr  # real exponent — fall through


# ---------------------------------------------------------------------------
# atan2_handler
# ---------------------------------------------------------------------------


def atan2_handler(_vm: VM, expr: IRApply) -> IRNode:
    """``Atan2(y, x)`` → ``math.atan2(y, x)`` when both are numeric."""
    if len(expr.args) != 2:
        return expr
    y, x = expr.args
    vy, vx = _to_number(y), _to_number(x)
    if vy is not None and vx is not None:
        return _from_number(_math.atan2(float(vy), float(vx)))
    return expr


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
        # Complex arithmetic — multiplication, division, integer powers.
        "ComplexMul": complex_mul_handler,
        "ComplexDiv": complex_div_handler,
        "_ComplexPow": complex_pow_handler,
        # Euler's formula and related.
        "ExpComplex": exp_complex_handler,
        "_EulerPow": euler_pow_handler,
        "Atan2": atan2_handler,
    }


# Expose the imaginary-power hook so SymbolicBackend can install it.
IMAGINARY_POWER_HOOK = imaginary_power_handler

#: Pre-bound constant exported for backend integration.
IMAGINARY_UNIT = IMAGINARY_UNIT
