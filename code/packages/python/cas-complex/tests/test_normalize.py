"""Tests for rectangular-form normalization."""
from __future__ import annotations

from symbolic_ir import ADD, MUL, NEG, IRApply, IRInteger, IRSymbol

from cas_complex.constants import IMAGINARY_UNIT
from cas_complex.normalize import (
    contains_imaginary,
    normalize_complex,
    split_rect,
)

x = IRSymbol("x")
y = IRSymbol("y")


def _mul(a: object, b: object) -> IRApply:
    return IRApply(MUL, (a, b))  # type: ignore[arg-type]


def _add(a: object, b: object) -> IRApply:
    return IRApply(ADD, (a, b))  # type: ignore[arg-type]


def _neg(a: object) -> IRApply:
    return IRApply(NEG, (a,))  # type: ignore[arg-type]


def test_contains_imaginary_true() -> None:
    assert contains_imaginary(IMAGINARY_UNIT) is True
    assert contains_imaginary(_mul(IRInteger(3), IMAGINARY_UNIT)) is True


def test_contains_imaginary_false() -> None:
    assert contains_imaginary(x) is False
    assert contains_imaginary(IRInteger(5)) is False


def test_split_rect_pure_imaginary() -> None:
    # ImaginaryUnit → (0, 1)
    real, imag = split_rect(IMAGINARY_UNIT)
    assert real == IRInteger(0)
    assert imag == IRInteger(1)


def test_split_rect_neg_imaginary() -> None:
    # Neg(ImaginaryUnit) → (0, -1)
    real, imag = split_rect(_neg(IMAGINARY_UNIT))
    assert real == IRInteger(0)
    assert imag == IRInteger(-1)


def test_split_rect_mul_imaginary() -> None:
    # 3 * i → (0, 3)
    node = _mul(IRInteger(3), IMAGINARY_UNIT)
    real, imag = split_rect(node)
    assert real == IRInteger(0)
    assert imag == IRInteger(3)


def test_split_rect_add_real_imag() -> None:
    # 5 + 3*i → (5, 3)
    node = _add(IRInteger(5), _mul(IRInteger(3), IMAGINARY_UNIT))
    real, imag = split_rect(node)
    assert real == IRInteger(5)
    assert imag == IRInteger(3)


def test_split_rect_pure_real() -> None:
    # x (no ImaginaryUnit) → (x, 0)
    real, imag = split_rect(x)
    assert real == x
    assert imag == IRInteger(0)


def test_normalize_complex_pure_imaginary() -> None:
    # ImaginaryUnit stays as ImaginaryUnit
    assert normalize_complex(IMAGINARY_UNIT) == IMAGINARY_UNIT


def test_normalize_complex_3i() -> None:
    # 3*i
    node = _mul(IRInteger(3), IMAGINARY_UNIT)
    result = normalize_complex(node)
    # Should be Mul(3, ImaginaryUnit)
    assert isinstance(result, IRApply)
    assert result.head.name == "Mul"


def test_normalize_complex_real_unchanged() -> None:
    assert normalize_complex(x) == x
    assert normalize_complex(IRInteger(42)) == IRInteger(42)


def test_normalize_complex_rect() -> None:
    # 2 + 3*i → Add(2, Mul(3, ImaginaryUnit))
    node = _add(IRInteger(2), _mul(IRInteger(3), IMAGINARY_UNIT))
    result = normalize_complex(node)
    # Should be an Add expression
    assert isinstance(result, IRApply)
    assert result.head.name in ("Add", "Mul")
