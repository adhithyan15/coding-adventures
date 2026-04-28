"""Re, Im, Conjugate operations on IR complex expressions."""
from __future__ import annotations

from symbolic_ir import IRInteger, IRNode, IRRational

from cas_complex.constants import IMAGINARY_UNIT, make_add, make_mul, make_neg
from cas_complex.normalize import contains_imaginary, normalize_complex, split_rect


def re_part(node: IRNode) -> IRNode:
    """Extract the real part of ``node``.

    If ``node`` contains no ``ImaginaryUnit``, it is already real —
    return it as-is.  Otherwise normalize to rectangular form and
    return the real component.
    """
    if not contains_imaginary(node):
        return node
    real, _imag = split_rect(normalize_complex(node))
    return real


def im_part(node: IRNode) -> IRNode:
    """Extract the imaginary coefficient of ``node``.

    Returns the scalar ``b`` such that the imaginary part is
    ``b * ImaginaryUnit``.  Returns ``IRInteger(0)`` if there is no
    imaginary component.
    """
    if not contains_imaginary(node):
        return IRInteger(0)
    _real, imag = split_rect(normalize_complex(node))
    return imag


def conjugate(node: IRNode) -> IRNode:
    """Return the complex conjugate of ``node``.

    ``conjugate(a + b*i) = a - b*i``.

    If ``node`` is purely real, returns it unchanged.
    """
    if not contains_imaginary(node):
        return node
    real, imag = split_rect(normalize_complex(node))

    # Negate the imaginary coefficient
    if isinstance(imag, IRInteger):
        neg_imag_coeff: IRNode = IRInteger(-imag.value)
    elif isinstance(imag, IRRational):
        neg_imag_coeff = IRRational(-imag.numer, imag.denom)
    else:
        neg_imag_coeff = make_neg(imag)

    # Check for zero imaginary part
    if isinstance(neg_imag_coeff, IRInteger) and neg_imag_coeff.value == 0:
        return real

    # Build imaginary term
    if isinstance(neg_imag_coeff, IRInteger) and neg_imag_coeff.value == 1:
        neg_imag_term: IRNode = IMAGINARY_UNIT
    elif isinstance(neg_imag_coeff, IRInteger) and neg_imag_coeff.value == -1:
        neg_imag_term = make_neg(IMAGINARY_UNIT)
    else:
        neg_imag_term = make_mul(neg_imag_coeff, IMAGINARY_UNIT)

    # Check for zero real part
    if isinstance(real, IRInteger) and real.value == 0:
        return neg_imag_term

    return make_add(real, neg_imag_term)
