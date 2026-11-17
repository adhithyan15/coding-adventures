"""IIR (Infinite Impulse Response) filtering via Direct-Form-II Transposed.

Conforms to DSP03 Phase 4 and VER15 security/weakness matrix invariants:
- Direct-Form-II Transposed structure matching scipy.signal.lfilter.
- Safe division by a[0].
- Bounded order (MAX_ORDER = 1024) and sample count (MAX_SAMPLES = 16_777_216).
- CWE-20 finite float validation fail-closed.
- CWE-674 strictly iterative execution (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_filters.fir import (
    MAX_ORDER,
    MAX_SAMPLES,
    MAX_TAPS,
    EmptyKernelError,
    EmptySignalError,
    InvalidCoefficientError,
)


def iir(
    signal: Sequence[float], b: Sequence[float], a: Sequence[float]
) -> list[float]:
    """Compute IIR filtering using Direct-Form-II Transposed structure.

    signal: real input sequence of length N.
    b: numerator / feed-forward polynomial coefficients.
    a: denominator / feedback polynomial coefficients.

    Returns a real sequence of length N.
    """
    n = len(signal)
    if n == 0:
        raise EmptySignalError("filter signal must be non-empty")
    m = len(b)
    n_coef = len(a)
    if m == 0 or n_coef == 0:
        raise EmptyKernelError("filter coefficients b and a must be non-empty")

    if n > MAX_SAMPLES:
        raise InvalidCoefficientError(
            f"signal length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )
    if m > MAX_TAPS or n_coef > MAX_TAPS:
        raise InvalidCoefficientError(
            f"coefficient length exceeds MAX_TAPS ({MAX_TAPS})"
        )

    a0 = float(a[0])
    if a0 == 0.0 or not math.isfinite(a0):
        raise InvalidCoefficientError(
            f"a[0] must be non-zero and finite; got {a0}"
        )

    order = max(m, n_coef) - 1
    if order > MAX_ORDER:
        raise InvalidCoefficientError(
            f"filter order {order} exceeds MAX_ORDER ({MAX_ORDER})"
        )

    # Validate all floats are finite
    for x in signal:
        if not math.isfinite(float(x)):
            raise InvalidCoefficientError("non-finite float in signal")
    for coef in b:
        if not math.isfinite(float(coef)):
            raise InvalidCoefficientError("non-finite float in b")
    for coef in a:
        if not math.isfinite(float(coef)):
            raise InvalidCoefficientError("non-finite float in a")

    inv_a0 = 1.0 / a0
    b_scaled = [
        float(b[k]) * inv_a0 if k < m else 0.0 for k in range(order + 1)
    ]
    a_scaled = [
        float(a[k]) * inv_a0 if k < n_coef else 0.0 for k in range(order + 1)
    ]

    state = [0.0] * order
    out: list[float] = []

    for x in signal:
        x_val = float(x)
        y = b_scaled[0] * x_val + (state[0] if order > 0 else 0.0)
        out.append(y)

        if order > 0:
            for k in range(order - 1):
                state[k] = (
                    b_scaled[k + 1] * x_val
                    - a_scaled[k + 1] * y
                    + state[k + 1]
                )
            state[order - 1] = (
                b_scaled[order] * x_val - a_scaled[order] * y
            )

    return out
