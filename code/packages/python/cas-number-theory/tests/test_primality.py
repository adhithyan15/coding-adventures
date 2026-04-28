"""Tests for cas_number_theory.primality."""
from __future__ import annotations
import pytest
from cas_number_theory.primality import is_prime, next_prime, prev_prime


def test_is_prime_small() -> None:
    assert is_prime(2) is True
    assert is_prime(3) is True
    assert is_prime(4) is False
    assert is_prime(0) is False
    assert is_prime(1) is False


def test_is_prime_97() -> None:
    assert is_prime(97) is True
    assert is_prime(100) is False


def test_carmichael_number() -> None:
    # 561 = 3 * 11 * 17 — passes Fermat but not Miller-Rabin
    assert is_prime(561) is False


def test_mersenne_prime() -> None:
    # 2^31 - 1 = 2147483647
    assert is_prime(2_147_483_647) is True


def test_is_prime_large_composite() -> None:
    assert is_prime(2_147_483_648) is False  # 2^31


def test_next_prime_10() -> None:
    assert next_prime(10) == 11


def test_next_prime_13() -> None:
    assert next_prime(13) == 17


def test_next_prime_2() -> None:
    assert next_prime(1) == 2


def test_prev_prime_10() -> None:
    assert prev_prime(10) == 7


def test_prev_prime_3() -> None:
    assert prev_prime(3) == 2


def test_prev_prime_2_returns_none() -> None:
    assert prev_prime(2) is None


def test_prev_prime_below_2_returns_none() -> None:
    assert prev_prime(1) is None
