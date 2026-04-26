"""Polynomial helpers."""

from __future__ import annotations

from cas_factor import (
    content,
    degree,
    divide_linear,
    divisors,
    evaluate,
    normalize,
    primitive_part,
)


def test_normalize_strips_trailing_zeros() -> None:
    assert normalize([1, 2, 0, 0]) == [1, 2]


def test_normalize_zero_polynomial() -> None:
    assert normalize([0, 0]) == []


def test_degree() -> None:
    assert degree([1, 2, 3]) == 2
    assert degree([5]) == 0
    assert degree([]) == -1


def test_evaluate() -> None:
    """p(x) = 1 + 2x + 3x^2 → p(2) = 1 + 4 + 12 = 17."""
    assert evaluate([1, 2, 3], 2) == 17


def test_evaluate_at_zero() -> None:
    assert evaluate([5, 7, 9], 0) == 5


def test_content_simple() -> None:
    assert content([2, 4, 6]) == 2


def test_content_with_negatives() -> None:
    assert content([-6, 4, 2]) == 2


def test_content_of_zero() -> None:
    assert content([]) == 0


def test_primitive_part() -> None:
    assert primitive_part([2, 4, 6]) == [1, 2, 3]


def test_divide_linear() -> None:
    """(x^2 - 1) / (x - 1) = x + 1."""
    quotient = divide_linear([-1, 0, 1], 1)
    assert quotient == [1, 1]


def test_divide_linear_nontrivial_root() -> None:
    """(x^3 - 6x^2 + 11x - 6) / (x - 1) = x^2 - 5x + 6."""
    p = [-6, 11, -6, 1]
    assert divide_linear(p, 1) == [6, -5, 1]


def test_divisors() -> None:
    assert divisors(12) == [1, 2, 3, 4, 6, 12]


def test_divisors_negative() -> None:
    assert divisors(-12) == [1, 2, 3, 4, 6, 12]


def test_divisors_zero() -> None:
    assert divisors(0) == []
