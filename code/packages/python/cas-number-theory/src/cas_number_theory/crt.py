"""Chinese Remainder Theorem."""
from __future__ import annotations
import math


def chinese_remainder(remainders: list[int], moduli: list[int]) -> int | None:
    """Solve x ≡ r_i (mod m_i) for all i.

    Returns the unique solution in [0, prod(moduli)) if all moduli are
    pairwise coprime, or None otherwise.

    Raises ValueError if remainders and moduli have different lengths or
    if any modulus is <= 0.
    """
    if len(remainders) != len(moduli):
        raise ValueError("remainders and moduli must have equal length")
    if not moduli:
        return 0
    for m in moduli:
        if m <= 0:
            raise ValueError(f"all moduli must be positive, got {m}")

    x = remainders[0] % moduli[0]
    m = moduli[0]

    for r_i, m_i in zip(remainders[1:], moduli[1:]):
        if math.gcd(m, m_i) != 1:
            return None  # not pairwise coprime
        # inv = m^(-1) mod m_i
        inv = pow(m, -1, m_i)
        x = x + m * ((r_i - x) * inv % m_i)
        m *= m_i

    return x % m
