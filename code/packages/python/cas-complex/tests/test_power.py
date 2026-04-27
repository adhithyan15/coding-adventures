"""Tests for ImaginaryUnit power reduction."""
from __future__ import annotations

from symbolic_ir import IRInteger, IRSymbol

from cas_complex.constants import IMAGINARY_UNIT
from cas_complex.power import reduce_imaginary_power


def test_i_power_0() -> None:
    assert reduce_imaginary_power(0) == IRInteger(1)


def test_i_power_1() -> None:
    assert reduce_imaginary_power(1) == IMAGINARY_UNIT


def test_i_power_2() -> None:
    assert reduce_imaginary_power(2) == IRInteger(-1)


def test_i_power_3_is_neg_i() -> None:
    result = reduce_imaginary_power(3)
    # Should be Neg(ImaginaryUnit)
    from symbolic_ir import IRApply
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
    assert result.args[0] == IMAGINARY_UNIT


def test_i_power_4() -> None:
    assert reduce_imaginary_power(4) == IRInteger(1)


def test_i_power_7() -> None:
    # 7 mod 4 = 3 → -i
    result = reduce_imaginary_power(7)
    from symbolic_ir import IRApply
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"


def test_i_power_8() -> None:
    assert reduce_imaginary_power(8) == IRInteger(1)


def test_i_power_negative_2() -> None:
    # -2 mod 4 = 2 in Python → -1
    assert reduce_imaginary_power(-2) == IRInteger(-1)


def test_i_power_negative_1() -> None:
    # -1 mod 4 = 3 → -i
    result = reduce_imaginary_power(-1)
    from symbolic_ir import IRApply
    assert isinstance(result, IRApply)
    assert result.head.name == "Neg"
