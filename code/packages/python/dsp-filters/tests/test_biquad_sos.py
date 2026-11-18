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


def test_legacy_biquad_tiny_reciprocal_normalization_is_nonfinite() -> None:
    min_subnormal = float.fromhex("0x0.0000000000001p-1022")
    assert min_subnormal.hex() == "0x0.0000000000001p-1022"
    assert math.isfinite(min_subnormal)

    # The raw coefficients are finite. The legacy reciprocal normalization
    # overflows even though direct division of these equal values is exact.
    assert min_subnormal / min_subnormal == 1.0
    output = Biquad(min_subnormal, 0.0, 0.0, min_subnormal, 0.0, 0.0).process(
        [1.0]
    )
    assert len(output) == 1
    assert math.isinf(output[0])
    assert output[0] > 0.0

    identity = Biquad(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    assert identity.process([1.0]) == [1.0]


def test_legacy_biquad_output_and_hidden_delay_overflow() -> None:
    large_finite = float.fromhex("0x1.0000000000000p+1023")
    assert large_finite.hex() == "0x1.0000000000000p+1023"
    assert math.isfinite(large_finite)

    immediate = Biquad(2.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    immediate_output = immediate.process([large_finite])
    assert len(immediate_output) == 1
    assert math.isinf(immediate_output[0])
    assert immediate_output[0] > 0.0

    delayed = Biquad(0.0, 2.0, 0.0, 1.0, 0.0, 0.0)
    assert delayed.process([large_finite]) == [0.0]
    delayed_output = delayed.process([0.0])
    assert math.isinf(delayed_output[0])
    assert delayed_output[0] > 0.0
    delayed.reset()
    assert delayed.process([0.0]) == [0.0]


def test_legacy_biquad_failed_block_commits_prefix_and_reset_replays() -> None:
    delay = Biquad(0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
    assert delay.process([1.0, 2.0]) == [0.0, 0.0]

    with pytest.raises(InvalidCoefficientError):
        delay.process([3.0, math.nan, 4.0])

    # The valid prefix commits before NaN is rejected, and the suffix is not
    # processed. This is an observation of the current legacy API.
    assert delay.process([0.0, 0.0]) == [2.0, 3.0]

    delay.reset()
    assert delay.process([1.0, 2.0, 3.0]) == [0.0, 0.0, 1.0]


def test_legacy_sos_final_and_intermediate_overflow_are_distinct() -> None:
    large_finite = float.fromhex("0x1.0000000000000p+1023")
    identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    gain_two = [2.0, 0.0, 0.0, 1.0, 0.0, 0.0]

    final_overflow = sosfilt([gain_two], [large_finite])
    assert len(final_overflow) == 1
    assert math.isinf(final_overflow[0])
    assert final_overflow[0] > 0.0

    after_identity = sosfilt([identity, gain_two], [large_finite])
    assert len(after_identity) == 1
    assert math.isinf(after_identity[0])
    assert after_identity[0] > 0.0

    with pytest.raises(InvalidCoefficientError):
        sosfilt([gain_two, identity], [large_finite])
