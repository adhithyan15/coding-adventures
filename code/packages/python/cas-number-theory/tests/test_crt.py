"""Tests for cas_number_theory.crt."""
from __future__ import annotations
import pytest
from cas_number_theory.crt import chinese_remainder


def test_crt_simple() -> None:
    # x ≡ 2 (mod 3), x ≡ 3 (mod 5) → x = 8
    assert chinese_remainder([2, 3], [3, 5]) == 8


def test_crt_single() -> None:
    assert chinese_remainder([5], [7]) == 5


def test_crt_empty() -> None:
    assert chinese_remainder([], []) == 0


def test_crt_non_coprime_returns_none() -> None:
    assert chinese_remainder([0, 0], [4, 6]) is None  # gcd(4,6)=2


def test_crt_three_moduli() -> None:
    # x ≡ 1 (mod 2), x ≡ 2 (mod 3), x ≡ 3 (mod 5)
    result = chinese_remainder([1, 2, 3], [2, 3, 5])
    assert result is not None
    assert result % 2 == 1
    assert result % 3 == 2
    assert result % 5 == 3


def test_crt_mismatched_length() -> None:
    with pytest.raises(ValueError):
        chinese_remainder([1, 2], [3])
