"""Rectangular-form normalization: separate real and imaginary parts."""
from __future__ import annotations

from symbolic_ir import (
    ADD,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_complex.constants import _NEG, IMAGINARY_UNIT, make_add, make_mul


def contains_imaginary(node: IRNode) -> bool:
    """Return True if ``node`` contains ``ImaginaryUnit`` anywhere."""
    if isinstance(node, IRSymbol) and node.name == "ImaginaryUnit":
        return True
    if isinstance(node, IRApply):
        return any(contains_imaginary(a) for a in node.args)
    return False


def _is_zero(node: IRNode) -> bool:
    """Return True if ``node`` is a numeric zero.

    Handles ``IRInteger(0)``, ``IRRational(0/n)``, and ``IRFloat(0.0)``
    — the last arises when the VM evaluates ``Sub(Mul(0, x), Mul(y, 0))``
    and returns a floating-point zero rather than an integer zero.
    """
    if isinstance(node, IRInteger):
        return node.value == 0
    if isinstance(node, IRRational):
        return node.numer == 0
    if isinstance(node, IRFloat):
        return node.value == 0.0
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
                    return IRRational(-c.numer, c.denom)
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

    # Sub(a, b) — treat as Add(a, Neg(b)) and recurse.
    if (
        isinstance(node, IRApply)
        and isinstance(node.head, IRSymbol)
        and node.head.name == "Sub"
        and len(node.args) == 2
    ):
        a, b = node.args
        return split_rect(IRApply(ADD, (a, IRApply(IRSymbol("Neg"), (b,)))))

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


def _clean_float_zero(node: IRNode) -> IRNode:
    """Replace a near-zero ``IRFloat`` with ``IRInteger(0)``.

    IEEE-754 arithmetic produces values like ``1.2246e-16`` when computing
    ``sin(π)`` — analytically zero but non-zero at machine precision.
    Cleaning these up avoids printing ``-1.0 + 1.2e-16*%i`` for ``e^(i*π)``.
    The threshold (1e-10) is intentionally loose: it matches Maxima's default
    ``fpprec`` behaviour and is safe for all values that arise from CAS-level
    trig/exp evaluation.  Raw numeric computation should use IRFloat arithmetic
    directly and never reaches this function.
    """
    if isinstance(node, IRFloat) and abs(node.value) < 1e-10:
        return IRInteger(0)
    return node


def normalize_complex(node: IRNode) -> IRNode:
    """Rewrite ``node`` into rectangular form ``a + b * ImaginaryUnit``.

    If there is no imaginary component, returns ``node`` unchanged.
    If the imaginary part is zero, returns only the real part.

    Near-zero ``IRFloat`` real and imaginary parts are rounded to zero
    to suppress floating-point noise from trig/exp evaluation (e.g.
    ``sin(π) ≈ 1.22e-16`` after Euler's formula should not produce a
    visible imaginary component).
    """
    if not contains_imaginary(node):
        return node

    real, imag = split_rect(node)

    # Clean up floating-point noise before deciding zero-ness.
    real = _clean_float_zero(real)
    imag = _clean_float_zero(imag)

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
