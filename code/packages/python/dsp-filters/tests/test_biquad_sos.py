"""Unit tests for Biquad and Second-Order Sections (SOS) cascade."""

from __future__ import annotations

import math

import pytest

from dsp_filters import (
    Biquad,
    InvalidCoefficientError,
    butterworth_lowpass,
    sos_cascade,
    sosfilt,
)


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_biquad_factories() -> None:
    bq_lp = Biquad.lowpass(0.1)
    bq_hp = Biquad.highpass(0.1)
    bq_bp = Biquad.bandpass(0.2)
    bq_notch = Biquad.notch(0.2)
    bq_allpass = Biquad.allpass(0.2)
    bq_peak = Biquad.peak(0.2, gain_db=3.0)

    sig = [1.0, 0.5, -0.5, 0.0, 1.0]
    for bq in [bq_lp, bq_hp, bq_bp, bq_notch, bq_allpass, bq_peak]:
        out = bq.process(sig)
        assert len(out) == len(sig)
        bq.reset()
        assert bq.z1 == 0.0
        assert bq.z2 == 0.0


def test_biquad_invalid_parameters() -> None:
    with pytest.raises(InvalidCoefficientError, match="cutoff_norm"):
        Biquad.lowpass(-0.1)
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.lowpass(0.1, q=-1.0)

    with pytest.raises(InvalidCoefficientError, match="cutoff_norm"):
        Biquad.highpass(0.6)
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.highpass(0.1, q=0.0)

    with pytest.raises(InvalidCoefficientError, match="center_norm"):
        Biquad.bandpass(0.0)
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.bandpass(0.2, q=-0.5)

    with pytest.raises(InvalidCoefficientError, match="center_norm"):
        Biquad.notch(0.5)
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.notch(0.2, q=0.0)

    with pytest.raises(InvalidCoefficientError, match="center_norm"):
        Biquad.allpass(-0.1)
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.allpass(0.2, q=-1.0)

    with pytest.raises(InvalidCoefficientError, match="center_norm"):
        Biquad.peak(0.6, gain_db=1.0)
    with pytest.raises(InvalidCoefficientError, match="gain_db"):
        Biquad.peak(0.2, gain_db=float("nan"))
    with pytest.raises(InvalidCoefficientError, match="Q"):
        Biquad.peak(0.2, gain_db=2.0, q=-1.0)

    # Invalid coefficient values in constructor
    with pytest.raises(InvalidCoefficientError, match="finite"):
        Biquad(float("nan"), 0.0, 0.0, 1.0, 0.0, 0.0)


def test_biquad_process_sample_matches_process() -> None:
    bq1 = Biquad.lowpass(0.15)
    bq2 = Biquad.lowpass(0.15)
    sig = [math.sin(0.2 * i) for i in range(20)]

    out_bulk = bq1.process(sig)
    out_sample = [bq2.process_sample(x) for x in sig]
    _assert_close(out_bulk, out_sample)


def test_sos_cascade_and_sosfilt() -> None:
    b, a = butterworth_lowpass(2, 0.2)
    sec = [b[0], b[1], b[2], a[0], a[1], a[2]]
    sos = [sec, sec]

    sig = [1.0, 0.5, -0.5, 0.2, 0.0] * 5
    out1 = sos_cascade(sig, sos)
    out2 = sosfilt(sos, sig)
    assert len(out1) == len(sig)
    _assert_close(out1, out2)


def test_sos_empty_cascade_is_identity() -> None:
    sig = [1.0, 2.0, 3.0]
    out = sos_cascade(sig, [])
    assert out == sig


def test_sos_invalid_section_length() -> None:
    with pytest.raises(InvalidCoefficientError, match="must have 6 coefficients"):
        sos_cascade([1.0, 2.0], [[1.0, 2.0, 3.0]])
