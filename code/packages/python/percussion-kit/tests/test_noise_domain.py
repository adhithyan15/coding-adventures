"""Pseudo-noise must stay finite for every sample index and seed.

`trig.sin` returns NaN beyond 2**29 radians (PHY00a 3.4.1).  The noise phase
`(i + 1) * (seed * 0.1618 + 0.618)` grows without bound, so it is reduced to
[0, 1) cycles first (PHY00a 7).  These are the seeds and counts at which the
unreduced phase used to fail.
"""

import math

import pytest

from percussion_kit.percussion_kit import _pseudo_noise

CASES = [
    (0, 10**9),  # seed 1e9 failed at sample 0
    (50_000, 12345),  # seed 12345 failed after about 43k samples
    (5_700_000, 89),  # the stock seed 89 failed at 29.6 s at 192 kHz
    (10**12, 89),
    (7, 2**32 - 1),
]


# The same values are asserted in the Rust and TypeScript lanes.
EXPECTED: list[float] = [
    0.06019736691951039,
    -0.578681446131668,
    -0.08452169376437269,
    -0.22354608627210837,
    -0.9668349004277843,
]


@pytest.mark.parametrize(("index", "seed"), CASES)
def test_pseudo_noise_is_finite_and_in_range(index: int, seed: int) -> None:
    value = _pseudo_noise(index, seed)
    assert math.isfinite(value)
    assert -1.0 <= value <= 1.0


def test_pseudo_noise_over_a_long_run_never_fails() -> None:
    for i in range(0, 200_000, 997):
        assert math.isfinite(_pseudo_noise(i, 12345))


def test_cross_lane_values() -> None:
    # The same values are asserted in the Rust and TypeScript lanes.
    for (index, seed), want in zip(CASES, EXPECTED, strict=True):
        assert _pseudo_noise(index, seed) == want


def test_non_finite_phase_is_nan_not_an_exception() -> None:
    # An overflowing phase gives NaN (as in the Rust and TypeScript lanes), not
    # the OverflowError math.floor would raise.
    assert math.isnan(_pseudo_noise(int(1e308), 10**9))
