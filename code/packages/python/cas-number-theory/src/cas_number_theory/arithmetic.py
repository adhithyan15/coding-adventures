"""Number-theoretic arithmetic: divisors, totient, Möbius, Jacobi."""
from __future__ import annotations
from cas_number_theory.factorize import factor_integer


def divisors(n: int) -> list[int]:
    """Return sorted list of all positive divisors of n.

    Raises ValueError for n <= 0.
    """
    if n <= 0:
        raise ValueError(f"divisors requires n > 0, got {n}")
    divs: set[int] = set()
    i = 1
    while i * i <= n:
        if n % i == 0:
            divs.add(i)
            divs.add(n // i)
        i += 1
    return sorted(divs)


def totient(n: int) -> int:
    """Euler's totient φ(n): count of integers in [1, n] coprime to n.

    Raises ValueError for n <= 0.
    """
    if n <= 0:
        raise ValueError(f"totient requires n > 0, got {n}")
    result = n
    for p, _ in factor_integer(n):
        result = result // p * (p - 1)
    return result


def moebius_mu(n: int) -> int:
    """Möbius function μ(n).

    Returns 0 if n has a squared prime factor,
    (-1)^k if n is a product of k distinct primes.
    Raises ValueError for n <= 0.
    """
    if n <= 0:
        raise ValueError(f"moebius_mu requires n > 0, got {n}")
    if n == 1:
        return 1
    factors = factor_integer(n)
    for _, exp in factors:
        if exp > 1:
            return 0
    k = len(factors)
    return (-1) ** k


def jacobi_symbol(a: int, n: int) -> int:
    """Jacobi symbol (a/n).

    n must be a positive odd integer.
    Returns -1, 0, or 1.
    Raises ValueError if n is even or n <= 0.
    """
    if n <= 0 or n % 2 == 0:
        raise ValueError(f"jacobi_symbol requires odd positive n, got {n}")
    a %= n
    result = 1
    while a != 0:
        while a % 2 == 0:
            a //= 2
            if n % 8 in (3, 5):
                result = -result
        a, n = n, a
        if a % 4 == 3 and n % 4 == 3:
            result = -result
        a %= n
    return result if n == 1 else 0


def integer_length(n: int, base: int = 10) -> int:
    """Number of digits of |n| in given base.

    integer_length(0, b) → 1 (zero has one digit).
    Raises ValueError for base < 2.
    """
    if base < 2:
        raise ValueError(f"integer_length requires base >= 2, got {base}")
    n = abs(n)
    if n == 0:
        return 1
    count = 0
    while n > 0:
        n //= base
        count += 1
    return count
