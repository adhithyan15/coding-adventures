"""Integer factorization: trial division + Pollard's rho."""
from __future__ import annotations
import math
import random
from cas_number_theory.primality import is_prime

_TRIAL_LIMIT = 1000


def _pollard_rho(n: int) -> int:
    """Find a non-trivial factor of n using Pollard's rho (Floyd cycle).

    Assumes n is composite. Returns a factor (not necessarily prime).
    """
    if n % 2 == 0:
        return 2
    for c in range(1, 100):
        x = random.randint(2, n - 1)
        y = x
        d = 1
        while d == 1:
            x = (x * x + c) % n
            y = (y * y + c) % n
            y = (y * y + c) % n
            d = math.gcd(abs(x - y), n)
        if d != n:
            return d
    return n  # failed — return n itself as fallback


def _factor_recursive(n: int, factors: list[tuple[int, int]], count: dict[int, int]) -> None:
    """Recursively factor n into primes, accumulating into count."""
    if n == 1:
        return
    if is_prime(n):
        count[n] = count.get(n, 0) + 1
        return
    # Try to find a factor
    d = _pollard_rho(n)
    _factor_recursive(d, factors, count)
    _factor_recursive(n // d, factors, count)


def factor_integer(n: int) -> list[tuple[int, int]]:
    """Return prime factorization of n as sorted list of (prime, exponent) pairs.

    factor_integer(12) → [(2, 2), (3, 1)]
    factor_integer(1)  → []
    Raises ValueError for n <= 0.
    """
    if n <= 0:
        raise ValueError(f"factor_integer requires n > 0, got {n}")
    if n == 1:
        return []

    count: dict[int, int] = {}

    # Phase 1: trial division by small primes
    from cas_number_theory.primality import _SMALL_PRIMES
    for p in _SMALL_PRIMES:
        if p * p > n:
            break
        while n % p == 0:
            count[p] = count.get(p, 0) + 1
            n //= p

    if n > 1:
        # Phase 2: Pollard's rho for the remaining cofactor
        _factor_recursive(n, [], count)

    return sorted(count.items())
