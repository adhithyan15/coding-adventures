"""Rectangular-form normalization: separate real and imaginary parts."""
from __future__ import annotations

from fractions import Fraction

from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRNode, IRRational, IRSymbol

from cas_complex.constants import IMAGINARY_UNIT, _NEG, make_add, make_mul


def contains_imaginary(node: IRNode) -> bool:
    """Return True if ``node`` contains ``ImaginaryUnit`` anywhere."""
    if isinstance(node, IRSymbol) and node.name == "ImaginaryUnit":
        return True
    if isinstance(node, IRApply):
        return any(contains_imaginary(a) for a in node.args)
    return False


def _is_zero(node: IRNode) -> bool:
    """Return True if ``node`` is the integer or rational zero."""
    if isinstance(node, IRInteger):
        return node.value == 0
    if isinstance(node, IRRational):
        return node.numerator == 0
    return False


def _imag_coefficient(node: IRNode) -> IRNode | None:
    """If ``node`` is of the form ``c * ImaginaryUnit`` (or just ``ImaginaryUnit``),
    return the coefficient ``c``; otherwise return ``None``.

    Recognises:
    - ``ImaginaryUnit``                → coefficient 1
    - ``Neg(ImaginaryUnit)``           → coefficient -1
    - ``Mul(c, ImaginaryUnit)``        → coefficient c
    - ``Mul(ImaginaryUnit, c)``        → coefficient c
    - ``Neg(Mul(c, ImaginaryUnit))``   → coefficient -c
    """
    if isinstance(node, IRSymbol) and node.name == "ImaginaryUnit":
        return IRInteger(1)

    if isinstance(node, IRApply):
        head_name = node.head.name if isinstance(node.head, IRSymbol) else ""

        # Neg(ImaginaryUnit)
        if head_name == "Neg" and len(node.args) == 1:
            inner = node.args[0]
            c = _imag_coefficient(inner)
            if c is not None:
                # negate c
                if isinstance(c, IRInteger):
                    return IRInteger(-c.value)
                if isinstance(c, IRRational):
                    return IRRational(-c.numerator, c.denominator)
                return IRApply(_NEG, (c,))

        # Mul(c, ImaginaryUnit) or Mul(ImaginaryUnit, c)
        if head_name == "Mul" and len(node.args) == 2:
            a, b = node.args
            if isinstance(a, IRSymbol) and a.name == "ImaginaryUnit":
                return b
            if isinstance(b, IRSymbol) and b.name == "ImaginaryUnit":
                return a

    return None


def split_rect(node: IRNode) -> tuple[IRNode, IRNode]:
    """Split ``node`` into ``(real_part, imag_part)`` where
    ``node ≡ real_part + imag_part * ImaginaryUnit``.

    If ``node`` has no imaginary component, ``imag_part`` is
    ``IRInteger(0)``.  If ``node`` is purely imaginary, ``real_part``
    is ``IRInteger(0)``.

    Handles ``Add(a, b, ...)`` by accumulating contributions, and
    delegates individual terms to ``_imag_coefficient``.
    """
    zero = IRInteger(0)

    # Pure ImaginaryUnit or Mul/Neg involving only ImaginaryUnit
    c = _imag_coefficient(node)
    if c is not None:
        return (zero, c)

    # Add(...) — walk terms
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Add"
    ):
        real_terms: list[IRNode] = []
        imag_terms: list[IRNode] = []
        for term in node.args:
            c2 = _imag_coefficient(term)
            if c2 is not None:
                imag_terms.append(c2)
            else:
                real_terms.append(term)

        def _sum(terms: list[IRNode]) -> IRNode:
            if not terms:
                return zero
            result = terms[0]
            for t in terms[1:]:
                result = IRApply(ADD, (result, t))
            return result

        return (_sum(real_terms), _sum(imag_terms))

    # Not obviously complex — treat as real
    return (node, zero)


def is_rect_form(node: IRNode) -> bool:
    """Return True if ``node`` can be split into real and imaginary parts."""
    return contains_imaginary(node)


def normalize_complex(node: IRNode) -> IRNode:
    """Rewrite ``node`` into rectangular form ``a + b * ImaginaryUnit``.

    If there is no imaginary component, returns ``node`` unchanged.
    If the imaginary part is zero, returns only the real part.
    """
    if not contains_imaginary(node):
        return node

    real, imag = split_rect(node)

    if _is_zero(imag):
        return real
    if _is_zero(real):
        if isinstance(imag, IRInteger) and imag.value == 1:
            return IMAGINARY_UNIT
        return make_mul(imag, IMAGINARY_UNIT)

    imag_term = (
        IMAGINARY_UNIT
        if isinstance(imag, IRInteger) and imag.value == 1
        else make_mul(imag, IMAGINARY_UNIT)
    )
    return make_add(real, imag_term)
