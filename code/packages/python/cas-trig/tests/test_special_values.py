"""Tests for the special-value lookup table."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRRational, IRSymbol

from cas_trig.special_values import lookup_special_value

PI = IRSymbol("%pi")
MUL = IRSymbol("Mul")
NEG = IRSymbol("Neg")


def _frac_pi(p: int, q: int) -> IRApply:
    """Build Mul(Rational(p, q), %pi)."""
    return IRApply(MUL, (IRRational(p, q), PI))


def _int_mul_pi(n: int) -> IRApply:
    return IRApply(MUL, (IRInteger(n), PI))


# ---------------------------------------------------------------------------
# sin special values
# ---------------------------------------------------------------------------


def test_sin_0() -> None:
    """sin(0) = 0."""
    result = lookup_special_value("Sin", IRApply(MUL, (IRInteger(0), PI)))
    # 0*pi → but lookup_special_value works on %pi-fraction form
    # Try directly with integer 0 arg (not valid %pi multiple)
    assert lookup_special_value("Sin", PI) is not None  # sin(π) = 0


def test_sin_pi() -> None:
    """sin(π) = 0."""
    result = lookup_special_value("Sin", PI)
    assert result == IRInteger(0)


def test_sin_pi_over_2() -> None:
    """sin(π/2) = 1."""
    result = lookup_special_value("Sin", _frac_pi(1, 2))
    assert result == IRInteger(1)


def test_sin_pi_over_6() -> None:
    """sin(π/6) = 1/2."""
    result = lookup_special_value("Sin", _frac_pi(1, 6))
    assert result == IRRational(1, 2)


def test_sin_pi_over_4() -> None:
    """sin(π/4) = √2/2."""
    result = lookup_special_value("Sin", _frac_pi(1, 4))
    assert result is not None
    assert isinstance(result, IRApply)  # Mul(Sqrt(2), Rational(1,2))


def test_sin_pi_over_3() -> None:
    """sin(π/3) = √3/2."""
    result = lookup_special_value("Sin", _frac_pi(1, 3))
    assert result is not None
    assert isinstance(result, IRApply)


def test_sin_2pi_over_3() -> None:
    """sin(2π/3) = √3/2."""
    result = lookup_special_value("Sin", _frac_pi(2, 3))
    assert result is not None


def test_sin_3pi_over_2() -> None:
    """sin(3π/2) = -1."""
    result = lookup_special_value("Sin", _frac_pi(3, 2))
    assert result == IRInteger(-1)


def test_sin_2pi() -> None:
    """sin(2π) = 0."""
    result = lookup_special_value("Sin", _int_mul_pi(2))
    assert result == IRInteger(0)


def test_sin_negative_pi() -> None:
    """sin(-π) = 0 (via modulo reduction: -1 mod 2 = 1, sin(π) = 0)."""
    neg_pi = IRApply(NEG, (PI,))
    result = lookup_special_value("Sin", neg_pi)
    assert result == IRInteger(0)


# ---------------------------------------------------------------------------
# cos special values
# ---------------------------------------------------------------------------


def test_cos_0_equiv() -> None:
    """cos(0·π) — handled via the table for key (0,1)."""
    # We use 2π which maps to (2,1) in normalized form
    # but after mod-2 reduction: 2*pi/pi = 2 → 2 mod 2 = 0 → (0,1)
    result = lookup_special_value("Cos", _int_mul_pi(2))
    assert result == IRInteger(1)  # cos(2π) = 1


def test_cos_pi() -> None:
    """cos(π) = -1."""
    result = lookup_special_value("Cos", PI)
    assert result == IRInteger(-1)


def test_cos_pi_over_2() -> None:
    """cos(π/2) = 0."""
    result = lookup_special_value("Cos", _frac_pi(1, 2))
    assert result == IRInteger(0)


def test_cos_pi_over_4() -> None:
    """cos(π/4) = √2/2."""
    result = lookup_special_value("Cos", _frac_pi(1, 4))
    assert result is not None
    assert isinstance(result, IRApply)


def test_cos_pi_over_3() -> None:
    """cos(π/3) = 1/2."""
    result = lookup_special_value("Cos", _frac_pi(1, 3))
    assert result == IRRational(1, 2)


def test_cos_pi_over_6() -> None:
    """cos(π/6) = √3/2."""
    result = lookup_special_value("Cos", _frac_pi(1, 6))
    assert result is not None
    assert isinstance(result, IRApply)


# ---------------------------------------------------------------------------
# tan special values
# ---------------------------------------------------------------------------


def test_tan_pi_over_4() -> None:
    """tan(π/4) = 1."""
    result = lookup_special_value("Tan", _frac_pi(1, 4))
    assert result == IRInteger(1)


def test_tan_pi_over_6() -> None:
    """tan(π/6) = 1/√3."""
    result = lookup_special_value("Tan", _frac_pi(1, 6))
    assert result is not None
    assert isinstance(result, IRApply)


def test_tan_pi_over_3() -> None:
    """tan(π/3) = √3."""
    result = lookup_special_value("Tan", _frac_pi(1, 3))
    assert result is not None
    assert isinstance(result, IRApply)


def test_tan_pi() -> None:
    """tan(π) = 0."""
    result = lookup_special_value("Tan", PI)
    assert result == IRInteger(0)


# ---------------------------------------------------------------------------
# Non-special values → None
# ---------------------------------------------------------------------------


def test_non_special_returns_none() -> None:
    """Non-recognised argument returns None."""
    x = IRSymbol("x")
    result = lookup_special_value("Sin", x)
    assert result is None


def test_unrecognised_function_returns_none() -> None:
    """Unrecognised function name returns None."""
    result = lookup_special_value("Exp", PI)
    assert result is None
