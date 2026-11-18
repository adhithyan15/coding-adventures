"""Regression and bounded input checks for FIR designer normalization."""

from __future__ import annotations

import math
import random
import struct

import pytest

from dsp_filters import (
    InvalidCoefficientError,
    WindowType,
    design_band_pass,
    design_high_pass,
    design_low_pass,
)

WINDOWS = (
    WindowType.RECTANGULAR,
    WindowType.HAMMING,
    WindowType.HANN,
    WindowType.BLACKMAN,
)
TAP_COUNTS = (1, 3, 5)
MIN_SUBNORMAL = float.fromhex("0x0.0000000000001p-1022")


def _from_bits(bits: int) -> float:
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def _bits(value: float) -> int:
    return int.from_bytes(struct.pack(">d", value), "big")


def _assert_finite(coefficients: list[float]) -> None:
    assert all(math.isfinite(coefficient) for coefficient in coefficients)


def _exercise(cutoff: float, taps: int, window: WindowType) -> None:
    _assert_finite(design_low_pass(cutoff, taps, window))
    _assert_finite(design_high_pass(cutoff, taps, window))
    _assert_finite(design_band_pass(0.0, cutoff, taps, window))


def test_reciprocal_overflow_uses_finite_normalized_coefficients() -> None:
    reciprocal_overflow_cutoff = _from_bits(0x0002000000000000)  # 2**-1025
    next_cutoff = _from_bits(0x0002000000000001)

    assert design_low_pass(reciprocal_overflow_cutoff, 1, "rectangular") == [1.0]
    assert design_band_pass(0.0, reciprocal_overflow_cutoff, 1, "rectangular") == [1.0]
    assert design_low_pass(next_cutoff, 1, "rectangular") == [1.0]
    assert design_high_pass(reciprocal_overflow_cutoff, 1, "rectangular") == [0.0]

    minsub_hann = design_low_pass(MIN_SUBNORMAL, 3, WindowType.HANN)
    assert minsub_hann == [0.0, 1.0, 0.0]
    _assert_finite(design_high_pass(MIN_SUBNORMAL, 3, WindowType.HANN))
    _assert_finite(design_band_pass(0.0, MIN_SUBNORMAL, 3, WindowType.HANN))


def test_finite_reciprocal_path_preserves_legacy_result_bits() -> None:
    low_pass = design_low_pass(49.0 / 65536.0, 1, WindowType.RECTANGULAR)
    assert _bits(low_pass[0]) == 0x3FEFFFFFFFFFFFFF  # nextDown(1)

    band_pass = design_band_pass(0.0, 49.0 / 65536.0, 1, WindowType.RECTANGULAR)
    assert _bits(band_pass[0]) == 0x3FEFFFFFFFFFFFFF  # nextDown(1)

    high_pass = design_high_pass(2.0**-53, 1, WindowType.RECTANGULAR)
    assert _bits(high_pass[0]) == 0x0000000000000000


def test_normalizer_zero_and_invalid_cutoff_behavior_is_unchanged() -> None:
    assert design_low_pass(0.0, 3, WindowType.RECTANGULAR) == [0.0, 0.0, 0.0]
    with pytest.raises(InvalidCoefficientError):
        design_band_pass(0.0, 0.0, 3, WindowType.RECTANGULAR)
    with pytest.raises(InvalidCoefficientError):
        design_low_pass(math.nan, 3, WindowType.RECTANGULAR)


def test_bounded_subnormal_corpus_is_finite() -> None:
    calls = 0
    for mantissa in range(1, 257):
        cutoff = mantissa * MIN_SUBNORMAL
        for taps in TAP_COUNTS:
            for window in WINDOWS:
                _exercise(cutoff, taps, window)
                calls += 3
    assert calls == 9216


def test_deterministic_raw_bit_boundary_property_cases() -> None:
    center_bits = 0x0002000000000000
    checked = 0
    for offset in range(-64, 65):
        cutoff = _from_bits(center_bits + offset)
        taps = TAP_COUNTS[(checked // 3) % len(TAP_COUNTS)]
        window = WINDOWS[(checked // 3) % len(WINDOWS)]
        _exercise(cutoff, taps, window)
        checked += 3
    assert checked == 387


@pytest.mark.parametrize("seed", [0xA0D10, 0x51BADA55])
def test_seeded_subnormal_fuzz(seed: int) -> None:
    rng = random.Random(seed)
    for _ in range(64):
        # Positive finite binary64 subnormal encoded directly as a raw word.
        cutoff = _from_bits(rng.randrange(1, 1 << 52))
        taps = rng.choice(TAP_COUNTS)
        window = rng.choice(WINDOWS)
        _exercise(cutoff, taps, window)
