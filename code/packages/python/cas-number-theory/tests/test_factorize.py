"""Tests for cas_number_theory.factorize."""
from __future__ import annotations
import math
import pytest
from cas_number_theory.factorize import factor_integer


def test_factor_integer_1() -> None:
    assert factor_integer(1) == []


def test_factor_integer_12() -> None:
    assert factor_integer(12) == [(2, 2), (3, 1)]


def test_factor_integer_360() -> None:
    assert factor_integer(360) == [(2, 3), (3, 2), (5, 1)]


def test_factor_integer_prime() -> None:
    assert factor_integer(97) == [(97, 1)]


def test_factor_integer_large_semiprime() -> None:
    # 2^32 + 1 = 4294967297 = 641 * 6700417
    n = 2**32 + 1
    factors = factor_integer(n)
    product = math.prod(p**e for p, e in factors)
    assert product == n


def test_factor_integer_invalid() -> None:
    with pytest.raises(ValueError):
        factor_integer(0)
    with pytest.raises(ValueError):
        factor_integer(-5)


def test_factor_integer_stress() -> None:
    """Verify factorization correctness for 100 random-ish values."""
    for n in range(2, 500):
        factors = factor_integer(n)
        product = math.prod(p**e for p, e in factors)
        assert product == n, f"factor_integer({n}) gave wrong product"
