"""cas-algebraic — polynomial factoring over algebraic number fields Q[√d].

This package extends the MACSYMA CAS system with the ability to factor
univariate polynomials over quadratic algebraic extensions of the rationals.

Quick start::

    from cas_algebraic import factor_over_extension

    # x^4 + 1 splits over Q[√2]
    factors = factor_over_extension([1, 0, 0, 0, 1], d=2)
    # Each factor is a list of (rational_part, radical_part) coefficient pairs.

    # x^2 - 2 splits over Q[√2] as (x - √2)(x + √2)
    factors = factor_over_extension([-2, 0, 1], d=2)

    # x^2 - 5 splits over Q[√5] as (x - √5)(x + √5)
    factors = factor_over_extension([-5, 0, 1], d=5)

VM integration::

    from cas_algebraic import build_alg_factor_handler_table
    # Returns {"AlgFactor": alg_factor_handler} for merging into the VM.

MACSYMA surface syntax (after wiring ``algfactor`` in the name table)::

    algfactor(x^4 + 1, sqrt(2))
    → (x^2 + sqrt(2)*x + 1) * (x^2 - sqrt(2)*x + 1)
"""

from cas_algebraic.algebraic import (
    AlgCoeff,
    AlgPoly,
    factor_over_extension,
)
from cas_algebraic.handlers import (
    alg_factor_handler,
    build_alg_factor_handler_table,
)

__all__ = [
    "AlgCoeff",
    "AlgPoly",
    "alg_factor_handler",
    "build_alg_factor_handler_table",
    "factor_over_extension",
]
