"""Unit tests for Continuous Wavelet Transform (cwt)."""

from __future__ import annotations

import math

import pytest

from dsp_wavelets.cwt import cwt
from dsp_wavelets.errors import EmptySignalError, InvalidParamError


def test_cwt_shape_and_values() -> None:
    sig = [1.0] * 32
    scales = [1.0, 2.0, 4.0]
    out = cwt(sig, scales, wavelet="morlet")
    assert len(out) == len(scales) * len(sig)
    for val in out:
        assert math.isfinite(val)
        assert val >= 0.0


def test_cwt_ricker_impulse() -> None:
    # A single centered impulse in a zero signal
    n = 31
    sig = [0.0] * n
    sig[n // 2] = 10.0
    scales = [2.0]
    out = cwt(sig, scales, wavelet="ricker")
    assert len(out) == n
    # The center should have the highest positive magnitude
    center_val = out[n // 2]
    assert center_val > 0.0
    assert center_val > out[0]
    assert center_val > out[-1]


def test_cwt_morlet_sine_response() -> None:
    # Morlet central frequency is omega_0 = 6.0 rad/sample / scale.
    # At scale s = 6.0, the target frequency is 1.0 rad/sample.
    omega = 1.0
    sig = [math.sin(omega * t) for t in range(64)]
    scales = [2.0, 6.0, 15.0]
    out = cwt(sig, scales, wavelet="morlet")
    assert len(out) == 3 * 64

    # The energy in the middle for scale s=6 should be greater than for s=15
    mid = 32
    val_s2 = out[0 * 64 + mid]
    val_s6 = out[1 * 64 + mid]
    val_s15 = out[2 * 64 + mid]
    assert val_s6 > val_s15
    assert val_s6 > val_s2


def test_cwt_invalid_inputs() -> None:
    with pytest.raises(EmptySignalError):
        cwt([], [1.0])

    with pytest.raises(InvalidParamError):
        cwt([1.0, 2.0], [])

    with pytest.raises(InvalidParamError):
        cwt([1.0, 2.0], [0.0])  # Non-positive scale

    with pytest.raises(InvalidParamError):
        cwt([1.0, 2.0], [-1.0])  # Negative scale

    with pytest.raises(InvalidParamError):
        cwt([1.0, 2.0], [float("inf")])

    with pytest.raises(InvalidParamError):
        cwt([1.0, 2.0], [1.0], wavelet="haar")  # Haar not supported in CWT
