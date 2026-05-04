"""Tests for cas_number_theory.arithmetic."""
from __future__ import annotations
import pytest
from cas_number_theory.arithmetic import divisors, totient, moebius_mu, jacobi_symbol, integer_length


def test_divisors_1() -> None:
    assert divisors(1) == [1]


def test_divisors_12() -> None:
    assert divisors(12) == [1, 2, 3, 4, 6, 12]


def test_divisors_sorted() -> None:
    assert divisors(30) == sorted(divisors(30))


def test_totient_1() -> None:
    assert totient(1) == 1


def test_totient_prime() -> None:
    for p in [2, 3, 5, 7, 11, 13]:
        assert totient(p) == p - 1


def test_totient_12() -> None:
    assert totient(12) == 4


def test_moebius_1() -> None:
    assert moebius_mu(1) == 1


def test_moebius_prime() -> None:
    assert moebius_mu(2) == -1
    assert moebius_mu(3) == -1


def test_moebius_squared() -> None:
    assert moebius_mu(4) == 0  # 2^2
    assert moebius_mu(9) == 0  # 3^2


def test_moebius_product_two_primes() -> None:
    assert moebius_mu(6) == 1  # 2*3 → (-1)^2 = 1


def test_jacobi_symbol() -> None:
    # (2/3) = -1 (2 is a non-residue mod 3)
    assert jacobi_symbol(2, 3) == -1
    # (1/n) = 1
    assert jacobi_symbol(1, 5) == 1
    # (0/n) = 0
    assert jacobi_symbol(0, 5) == 0


def test_jacobi_symbol_invalid() -> None:
    with pytest.raises(ValueError):
        jacobi_symbol(2, 4)  # even modulus


def test_integer_length_decimal() -> None:
    assert integer_length(0) == 1
    assert integer_length(9) == 1
    assert integer_length(10) == 2
    assert integer_length(100) == 3


def test_integer_length_binary() -> None:
    assert integer_length(8, 2) == 4  # 1000


def test_integer_length_negative() -> None:
    assert integer_length(-100) == 3
