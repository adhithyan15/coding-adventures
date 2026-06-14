"""cas-number-theory: integer number theory operations."""
from __future__ import annotations

from cas_number_theory.primality import is_prime, next_prime, prev_prime
from cas_number_theory.factorize import factor_integer
from cas_number_theory.arithmetic import (
    divisors,
    integer_length,
    jacobi_symbol,
    moebius_mu,
    totient,
)
from cas_number_theory.crt import chinese_remainder
from cas_number_theory.handlers import build_number_theory_handler_table

__all__ = [
    "is_prime",
    "next_prime",
    "prev_prime",
    "factor_integer",
    "divisors",
    "totient",
    "moebius_mu",
    "jacobi_symbol",
    "chinese_remainder",
    "integer_length",
    "build_number_theory_handler_table",
]
