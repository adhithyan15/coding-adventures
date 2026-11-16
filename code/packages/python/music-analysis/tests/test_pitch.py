"""Unit tests for YIN and HPS pitch detection (MUS08, VER15)."""

from __future__ import annotations

import math

import pytest

from music_analysis import hps_pitch, yin_pitch, yin_pitch_track
from music_analysis.types import InvalidInputError


def _synthesize_sine(
    freq_hz: float, sample_rate_hz: float, duration_sec: float
) -> list[float]:
    n = int(sample_rate_hz * duration_sec)
    phase_inc = 2.0 * math.pi * freq_hz / sample_rate_hz
    return [math.sin(i * phase_inc) for i in range(n)]


def _synthesize_harmonic(
    f0: float,
    harmonics: list[float],
    sample_rate_hz: float,
    duration_sec: float,
) -> list[float]:
    n = int(sample_rate_hz * duration_sec)
    out = [0.0] * n
    for h, amp in enumerate(harmonics, start=1):
        freq = float(h) * f0
        if freq < sample_rate_hz / 2.0:
            phase_inc = 2.0 * math.pi * freq / sample_rate_hz
            for i in range(n):
                out[i] += amp * math.sin(i * phase_inc)
    return out


def test_yin_pitch_pure_sines() -> None:
    sr = 44100.0
    for target_freq in [110.0, 220.0, 440.0, 880.0]:
        sig = _synthesize_sine(target_freq, sr, duration_sec=0.1)
        est = yin_pitch(sig, sr, f_min=50.0, f_max=1200.0)
        assert math.isclose(est, target_freq, rel_tol=0.01)


def test_yin_pitch_harmonic_signal() -> None:
    sr = 44100.0
    target_f0 = 261.63  # C4
    # Fundamental + 2nd, 3rd, 4th harmonics
    sig = _synthesize_harmonic(target_f0, [1.0, 0.6, 0.4, 0.2], sr, duration_sec=0.1)
    est = yin_pitch(sig, sr, f_min=80.0, f_max=600.0)
    assert math.isclose(est, target_f0, rel_tol=0.01)


def test_yin_pitch_silence() -> None:
    sr = 44100.0
    sig = [0.0] * 2000
    est = yin_pitch(sig, sr)
    assert est == 0.0


def test_yin_pitch_track() -> None:
    sr = 44100.0
    # Two note sequence: 0.1s of 220 Hz followed by 0.1s of 440 Hz
    sig1 = _synthesize_sine(220.0, sr, duration_sec=0.1)
    sig2 = _synthesize_sine(440.0, sr, duration_sec=0.1)
    combined = sig1 + sig2

    track = yin_pitch_track(combined, sr, frame_length=1024, hop_length=256)
    assert isinstance(track, list)
    assert len(track) > 5

    # Early frames should detect near 220 Hz
    early_pitches = [p for p in track[:4] if p > 0.0]
    assert any(math.isclose(p, 220.0, rel_tol=0.02) for p in early_pitches)

    # Later frames should detect near 440 Hz
    late_pitches = [p for p in track[-4:] if p > 0.0]
    assert any(math.isclose(p, 440.0, rel_tol=0.02) for p in late_pitches)


def test_hps_pitch_harmonic_signal() -> None:
    sr = 44100.0
    target_f0 = 440.0
    sig = _synthesize_harmonic(
        target_f0, [1.0, 0.8, 0.6, 0.4, 0.2], sr, duration_sec=0.15
    )
    est = hps_pitch(sig, sr, f_min=100.0, f_max=1000.0)
    assert math.isclose(est, target_f0, rel_tol=0.02)


def test_yin_pitch_short_signal() -> None:
    sr = 44100.0
    # Very short signal where tau_max <= tau_min
    sig = [0.1, -0.1, 0.2, -0.2]
    assert yin_pitch(sig, sr) == 0.0

    # Short signal in pitch track
    track = yin_pitch_track([0.5] * 100, sr, frame_length=512, hop_length=128)
    assert len(track) == 1


def test_hps_pitch_silence() -> None:
    sr = 44100.0
    sig = [0.0] * 2048
    assert hps_pitch(sig, sr) == 0.0


def test_hps_pitch_explicit_fft() -> None:
    sr = 44100.0
    sig = _synthesize_harmonic(440.0, [1.0, 0.8, 0.6, 0.4], sr, 0.15)
    est = hps_pitch(sig, sr, n_fft=2048)
    assert math.isclose(est, 440.0, rel_tol=0.03)

    # Inverted min/max search bin condition
    assert hps_pitch(sig, sr, f_min=440.0, f_max=440.1, n_fft=512) == 0.0


def test_pitch_invalid_inputs() -> None:
    with pytest.raises(InvalidInputError):
        yin_pitch([], 44100.0)
    with pytest.raises(InvalidInputError):
        yin_pitch([1.0], -44100.0)
    with pytest.raises(InvalidInputError):
        yin_pitch([1.0] * 100, 44100.0, f_min=1000.0, f_max=500.0)
    with pytest.raises(InvalidInputError):
        hps_pitch([1.0] * 100, 44100.0, f_min=1000.0, f_max=500.0)
