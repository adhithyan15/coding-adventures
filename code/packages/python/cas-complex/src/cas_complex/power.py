"""ImaginaryUnit power reduction: i^n → one of {1, i, -1, -i}."""
from __future__ import annotations

from symbolic_ir import IRInteger, IRNode

from cas_complex.constants import IMAGINARY_UNIT, make_neg

# The four canonical values of i^n (n mod 4)
_I_POWERS: dict[int, IRNode] = {
    0: IRInteger(1),
    1: IMAGINARY_UNIT,
    2: IRInteger(-1),
    3: make_neg(IMAGINARY_UNIT),
}


def reduce_imaginary_power(n: int) -> IRNode:
    """Return the simplified value of ``ImaginaryUnit^n`` for integer ``n``.

    Applies the cyclic rule ``i^n = i^(n mod 4)``:

    - ``i^0 = 1``
    - ``i^1 = i``
    - ``i^2 = -1``
    - ``i^3 = -i``
    - ``i^4 = 1`` (and so on)

    Handles negative exponents via Python's ``%`` (floor division
    semantics), which gives the correct non-negative remainder.
    """
    return _I_POWERS[n % 4]
