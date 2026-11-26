"""Reusable sample medians and exact distribution-free median intervals."""

from collections.abc import Sequence
from math import isfinite
from numbers import Real

__all__ = ["median", "median_interval"]


def _ordered[T: Real](values: Sequence[T]) -> list[float]:
    if not values:
        raise ValueError("values must not be empty")
    result = []
    for value in values:
        if isinstance(value, bool) or not isinstance(value, Real):
            raise ValueError("values must be finite real numbers")
        try:
            value = float(value)
            finite = isfinite(value)
        except (OverflowError, TypeError):
            finite = False
        if not finite:
            raise ValueError("values must be finite real numbers")
        result.append(value)
    result.sort()
    return result


def median[T: Real](values: Sequence[T]) -> float:
    """Return the sample median without modifying ``values``.

    For an even sample, the central pair's midpoint is computed without
    overflowing for finite floating-point values near opposite limits.
    """
    ordered = _ordered(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    left, right = ordered[middle - 1], ordered[middle]
    # Avoid both overflow in left + right and in right - left.
    total = left + right
    return total / 2 if isfinite(total) else left / 2 + right / 2


def median_interval[T: Real](
    values: Sequence[T], confidence_percent: int
) -> tuple[float, float] | None:
    """Return the exact two-sided order-statistic interval for a population median.

    The binomial lower tail is accumulated using integer coefficients, so the
    rank cutoff is exact even at a confidence boundary. ``None`` means the
    sample is too small to provide an interval at the requested confidence.
    """
    if type(confidence_percent) is not int or not 1 <= confidence_percent <= 99:
        raise ValueError("confidence_percent must be an integer from 1 through 99")
    ordered = _ordered(values)
    n = len(ordered)
    # Compare 2 * tail / 2**n <= (100-confidence)/100 exactly.
    max_tail_scaled = (100 - confidence_percent) * (1 << n)
    coefficient = 1
    tail = 0
    rank = 0
    for j in range(n // 2 + 1):
        tail += coefficient
        if 200 * tail > max_tail_scaled:
            break
        rank = j + 1
        coefficient = coefficient * (n - j) // (j + 1)
    if rank == 0:
        return None
    return ordered[rank - 1], ordered[-rank]
