"""RectForm, PolarForm, Arg handlers."""
from __future__ import annotations

from symbolic_ir import IRApply, IRInteger, IRNode, IRSymbol

from cas_complex.constants import (
    _ATAN2,
    _EXP,
    _SQRT,
    IMAGINARY_UNIT,
    make_mul,
)
from cas_complex.normalize import contains_imaginary, normalize_complex, split_rect

_POW2 = IRInteger(2)
_HALF = IRSymbol("Rational")


def rect_form(node: IRNode) -> IRNode:
    """Rewrite ``node`` as ``a + b * ImaginaryUnit``.

    Delegates to :func:`normalize_complex` which already performs this
    rewriting.  If ``node`` is purely real, returns it unchanged.
    """
    return normalize_complex(node)


def arg(node: IRNode) -> IRNode:
    """Return the principal argument ``arctan2(Im(z), Re(z))``.

    Returns an unevaluated ``Atan2(im_part, re_part)`` IR expression.
    Numeric folding happens downstream via the VM.
    """
    if not contains_imaginary(node):
        # Real argument: Arg(x) = 0 if x > 0, π if x < 0
        # Return unevaluated for symbolic x.
        return IRApply(IRSymbol("Arg"), (node,))

    normalized = normalize_complex(node)
    real, imag = split_rect(normalized)
    return IRApply(_ATAN2, (imag, real))


def polar_form(node: IRNode) -> IRNode:
    """Rewrite ``node`` as ``r * Exp(ImaginaryUnit * theta)``.

    Returns ``Mul(r, Exp(Mul(ImaginaryUnit, theta)))`` IR expression.
    """
    if not contains_imaginary(node):
        # Real expression: r = |node|, theta = 0 or π.
        # Return unevaluated.
        return IRApply(IRSymbol("PolarForm"), (node,))

    normalized = normalize_complex(node)
    real, imag = split_rect(normalized)

    # r = Sqrt(a^2 + b^2)
    r: IRNode = IRApply(
        _SQRT,
        (
            IRApply(
                IRSymbol("Add"),
                (
                    IRApply(IRSymbol("Pow"), (real, _POW2)),
                    IRApply(IRSymbol("Pow"), (imag, _POW2)),
                ),
            ),
        ),
    )

    # theta = Atan2(b, a)
    theta: IRNode = IRApply(_ATAN2, (imag, real))

    # r * Exp(i * theta)
    return make_mul(r, IRApply(_EXP, (make_mul(IMAGINARY_UNIT, theta),)))
