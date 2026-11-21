"""Hostile input testing for `wave` (PHY01, VER15: CWE-20/190/400/674)."""

from __future__ import annotations

import math
import sys
from fractions import Fraction

import pytest
from trig import PI

from coding_adventures_wave import Wave


class TestCwe20TypeAndFloatValidation:
    @pytest.mark.parametrize(
        "bad_val",
        [
            True,
            False,
            float("nan"),
            float("inf"),
            float("-inf"),
            "1.0",
            "x" * 256,
            None,
            complex(1.0, 0.0),
            [],
            {},
        ],
    )
    def test_rejects_bool_non_finite_and_non_real_inputs(
        self, bad_val: object
    ) -> None:
        with pytest.raises(ValueError):
            Wave(amplitude=bad_val, frequency=1.0)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            Wave(amplitude=1.0, frequency=bad_val)  # type: ignore[arg-type]
        with pytest.raises(ValueError):
            Wave(amplitude=1.0, frequency=1.0, phase=bad_val)  # type: ignore[arg-type]
        w = Wave(amplitude=1.0, frequency=1.0)
        with pytest.raises(ValueError):
            w.evaluate(bad_val)  # type: ignore[arg-type]

    @pytest.mark.parametrize(
        "huge_real",
        [10**400, -(10**400), Fraction(10**400, 3), Fraction(-(10**400), 3)],
        ids=["int-pos", "int-neg", "frac-pos", "frac-neg"],
    )
    def test_huge_real_overflows_raise_value_error_not_overflow_error(
        self, huge_real: object
    ) -> None:
        with pytest.raises(ValueError, match="overflows float range"):
            Wave(amplitude=huge_real, frequency=1.0)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            Wave(amplitude=1.0, frequency=huge_real)  # type: ignore[arg-type]
        with pytest.raises(ValueError, match="overflows float range"):
            Wave(amplitude=1.0, frequency=1.0, phase=huge_real)  # type: ignore[arg-type]
        w = Wave(amplitude=1.0, frequency=1.0)
        with pytest.raises(ValueError, match="overflows float range"):
            w.evaluate(huge_real)  # type: ignore[arg-type]

    def test_rejects_boundary_domain_violations_and_normalizes_neg_zero(self) -> None:
        with pytest.raises(ValueError, match="Amplitude must be >= 0"):
            Wave(amplitude=-sys.float_info.min, frequency=1.0)
        with pytest.raises(ValueError, match="Frequency must be > 0"):
            Wave(amplitude=1.0, frequency=-0.0)
        with pytest.raises(ValueError, match="Angular frequency"):
            Wave(amplitude=1.0, frequency=(sys.float_info.max / (2.0 * PI)) * 1.0001)

        w = Wave(amplitude=-0.0, frequency=1.0, phase=-0.0)
        assert math.copysign(1.0, w.amplitude) == 1.0
        assert math.copysign(1.0, w.phase) == 1.0

    def test_corrupted_internal_state_fails_closed(self) -> None:
        w = Wave(amplitude=1.0, frequency=1.0)
        w._frequency = float("nan")
        with pytest.raises(ValueError, match="finite"):
            w.period()
        with pytest.raises(ValueError, match="finite"):
            w.angular_frequency()
        with pytest.raises(ValueError, match="finite"):
            w.evaluate(0.0)


class TestCwe190And400BoundedExecution:
    def test_bounded_error_message_length_for_oversized_inputs(self) -> None:
        with pytest.raises(ValueError) as exc_info:
            Wave(amplitude="A" * 100_000, frequency=1.0)  # type: ignore[arg-type]
        assert len(str(exc_info.value)) < 256

    def test_extreme_finite_and_subnormal_values_remain_bounded(self) -> None:
        subnormal = Wave(
            amplitude=1.0,
            frequency=float.fromhex("0x0.0000000000001p-1022"),
            phase=sys.float_info.max,
        )
        assert math.isinf(subnormal.period())
        times = (
            -sys.float_info.max,
            -1e100,
            -1.0,
            0.0,
            1.0,
            1e100,
            sys.float_info.max,
        )
        for t in times:
            y = subnormal.evaluate(t)
            assert math.isfinite(y)
            assert abs(y) <= 1.0

        huge = Wave(
            amplitude=sys.float_info.max,
            frequency=1e300,
            phase=-sys.float_info.max,
        )
        for t in (-sys.float_info.max, -1e300, 0.0, 1e300, sys.float_info.max):
            y = huge.evaluate(t)
            assert math.isfinite(y)
            assert abs(y) <= sys.float_info.max
