"""Tests for Re, Im, Conjugate."""
from __future__ import annotations

from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRSymbol

from cas_complex.constants import IMAGINARY_UNIT
from cas_complex.parts import conjugate, im_part, re_part

x = IRSymbol("x")


def _mul(a: object, b: object) -> IRApply:
    return IRApply(MUL, (a, b))  # type: ignore[arg-type]


def _add(a: object, b: object) -> IRApply:
    return IRApply(ADD, (a, b))  # type: ignore[arg-type]


def _rect(a: object, b: object) -> IRApply:
    """Build a + b*i."""
    return _add(a, _mul(b, IMAGINARY_UNIT))  # type: ignore[arg-type]


def test_re_of_pure_real() -> None:
    assert re_part(x) == x
    assert re_part(IRInteger(5)) == IRInteger(5)


def test_re_of_rect() -> None:
    node = _rect(IRInteger(3), IRInteger(4))
    assert re_part(node) == IRInteger(3)


def test_re_of_pure_imaginary() -> None:
    # Re(4*i) = 0
    node = _mul(IRInteger(4), IMAGINARY_UNIT)
    result = re_part(node)
    assert result == IRInteger(0)


def test_im_of_pure_real() -> None:
    assert im_part(x) == IRInteger(0)


def test_im_of_rect() -> None:
    node = _rect(IRInteger(3), IRInteger(4))
    assert im_part(node) == IRInteger(4)


def test_im_of_pure_imaginary() -> None:
    # Im(i) = 1
    assert im_part(IMAGINARY_UNIT) == IRInteger(1)


def test_conjugate_pure_real() -> None:
    assert conjugate(x) == x


def test_conjugate_rect() -> None:
    # conjugate(3 + 4i) = 3 - 4i
    node = _rect(IRInteger(3), IRInteger(4))
    result = conjugate(node)
    assert isinstance(result, IRApply)
    # Should be Add(3, Mul(-4, ImaginaryUnit)) or similar
    assert result.head.name in ("Add",)


def test_conjugate_pure_imaginary() -> None:
    # conjugate(i) = -i
    result = conjugate(IMAGINARY_UNIT)
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
    assert result.args[0] == IMAGINARY_UNIT
