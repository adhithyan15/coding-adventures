"""Primality testing, next/prev prime."""
from __future__ import annotations

# Pre-built sieve for n < SIEVE_LIMIT
SIEVE_LIMIT = 1_000_000

# Build the sieve at import time (125 KB bool array)
_sieve: list[bool] = [True] * SIEVE_LIMIT
_sieve[0] = _sieve[1] = False
for _p in range(2, int(SIEVE_LIMIT**0.5) + 1):
    if _sieve[_p]:
        for _j in range(_p * _p, SIEVE_LIMIT, _p):
            _sieve[_j] = False

# Primes from the sieve (for witness selection)
_SMALL_PRIMES: list[int] = [p for p in range(2, 1000) if _sieve[p]]


def _miller_rabin_test(n: int, a: int) -> bool:
    """Single Miller-Rabin witness test. Returns True if n is *probably* prime."""
    if n % a == 0:
        return n == a
    d, r = n - 1, 0
    while d % 2 == 0:
        d //= 2
        r += 1
    x = pow(a, d, n)
    if x == 1 or x == n - 1:
        return True
    for _ in range(r - 1):
        x = x * x % n
        if x == n - 1:
            return True
    return False


# Deterministic witnesses for n < 3_215_031_751
_WITNESSES_SMALL = [2, 3, 5, 7]
# BPSW-equivalent witnesses for larger n
_WITNESSES_LARGE = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71]


def is_prime(n: int) -> bool:
    """Return True iff n is prime.

    Uses a deterministic Miller-Rabin test for n < 3_215_031_751,
    and a BPSW-equivalent test (first 20 prime witnesses) for larger n.
    """
    if n < 2:
        return False
    if n < SIEVE_LIMIT:
        return _sieve[n]
    # Trial division by small primes first
    for p in _SMALL_PRIMES:
        if n == p:
            return True
        if n % p == 0:
            return False
    witnesses = _WITNESSES_SMALL if n < 3_215_031_751 else _WITNESSES_LARGE
    return all(_miller_rabin_test(n, a) for a in witnesses if a < n)


def next_prime(n: int) -> int:
    """Return the smallest prime strictly greater than n."""
    candidate = n + 1
    if candidate < 2:
        candidate = 2
    # Even → make odd (2 is the only even prime, handled separately)
    if candidate == 2:
        return 2
    if candidate % 2 == 0:
        candidate += 1
    while not is_prime(candidate):
        candidate += 2
    return candidate


def prev_prime(n: int) -> int | None:
    """Return the largest prime strictly less than n, or None if n <= 2."""
    if n <= 2:
        return None
    candidate = n - 1
    if candidate == 2:
        return 2
    if candidate % 2 == 0:
        candidate -= 1
    while candidate >= 2 and not is_prime(candidate):
        candidate -= 2
    return candidate if candidate >= 2 else None
