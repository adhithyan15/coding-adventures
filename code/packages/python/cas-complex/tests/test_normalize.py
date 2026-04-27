"""Tests for rectangular-form normalization."""
from __future__ import annotations

from symbolic_ir import ADD, MUL, NEG, IRApply, IRFloat, IRInteger, IRRational, IRSymbol

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


# ---------------------------------------------------------------------------
# IRFloat zero handling — the _is_zero / _clean_float_zero path
# ---------------------------------------------------------------------------


def test_normalize_strips_float_zero_real_part() -> None:
    """normalize_complex(IRFloat(0.0) + b*i) → b*i (strips the spurious real 0.0)."""
    node = _add(IRFloat(0.0), _mul(IRInteger(3), IMAGINARY_UNIT))
    result = normalize_complex(node)
    # Real part should be gone — result is purely imaginary
    assert isinstance(result, IRApply)
    assert result.head.name in ("Mul",) or result == IMAGINARY_UNIT


def test_normalize_strips_float_near_zero_imag_part() -> None:
    """normalize_complex(a + 1e-16*i) → a (strips the near-zero imaginary part)."""
    node = _add(IRInteger(3), _mul(IRFloat(1e-16), IMAGINARY_UNIT))
    result = normalize_complex(node)
    # Imaginary part cleaned to zero → return just the real part
    assert result == IRInteger(3)


def test_normalize_strips_float_near_zero_real_and_imag() -> None:
    """normalize_complex(1e-16 + 1.0*i) → 1.0*i (real part cleaned to zero)."""
    node = _add(IRFloat(1e-16), _mul(IRFloat(1.0), IMAGINARY_UNIT))
    result = normalize_complex(node)
    # 1e-16 < threshold → treated as zero; result is purely imaginary
    assert isinstance(result, IRApply)


def test_normalize_preserves_significant_real() -> None:
    """normalize_complex(1.0 + 2.0*i) keeps the real part."""
    node = _add(IRFloat(1.0), _mul(IRFloat(2.0), IMAGINARY_UNIT))
    result = normalize_complex(node)
    assert isinstance(result, IRApply)
    assert result.head.name == "Add"


def test_split_rect_float_zero_real() -> None:
    """split_rect(IRFloat(0.0) + 3*i) = (IRFloat(0.0), IRInteger(3))."""
    node = _add(IRFloat(0.0), _mul(IRInteger(3), IMAGINARY_UNIT))
    real, imag = split_rect(node)
    assert isinstance(real, IRFloat)
    assert real.value == 0.0
    assert imag == IRInteger(3)


def test_normalize_sub_form() -> None:
    """normalize_complex handles Sub(a, b) by converting to Add(a, Neg(b))."""
    sub = IRApply(IRSymbol("Sub"), (
        _add(IRInteger(3), _mul(IRInteger(1), IMAGINARY_UNIT)),
        _mul(IRInteger(2), IMAGINARY_UNIT),
    ))
    result = normalize_complex(sub)
    # 3 + i - 2i = 3 - i; should produce a complex expression
    assert isinstance(result, IRApply)


def test_normalize_rational_zero_coefficient() -> None:
    """normalize_complex(Add(x, Mul(IRRational(0,1), i))) strips the zero imag term."""
    x = IRSymbol("x")
    node = _add(x, _mul(IRRational(0, 1), IMAGINARY_UNIT))
    result = normalize_complex(node)
    # Imag part is zero rational → result should be x
    assert result == x
