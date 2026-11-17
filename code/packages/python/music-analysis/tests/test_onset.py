"""Unit tests for multi-resolution onset detection (MUS08, VER15)."""

from __future__ import annotations

import math

import pytest

from music_analysis import detect_onsets
from music_analysis.types import InvalidInputError


def _make_tone_burst(
    freq: float,
    duration_sec: float,
    sample_rate_hz: float,
) -> list[float]:
    n = int(sample_rate_hz * duration_sec)
    phase_inc = 2.0 * math.pi * freq / sample_rate_hz
    # Apply fast attack decay envelope
    out = [0.0] * n
    for i in range(n):
        t = i / sample_rate_hz
        env = math.exp(-15.0 * t)
        out[i] = env * math.sin(i * phase_inc)
    return out


def test_detect_onsets_single_burst() -> None:
    sr = 44100.0
    burst = _make_tone_burst(440.0, 0.2, sr)
    onsets = detect_onsets(burst, sr)
    assert len(onsets) >= 1
    # First onset should be near 0.0s
    assert math.isclose(onsets[0], 0.0, abs_tol=0.03)


def test_detect_onsets_multiple_events() -> None:
    sr = 44100.0
    silence = [0.0] * int(0.15 * sr)
    burst1 = _make_tone_burst(440.0, 0.1, sr)
    burst2 = _make_tone_burst(880.0, 0.1, sr)
    burst3 = _make_tone_burst(550.0, 0.1, sr)

    # 3 bursts separated by silence
    sig = burst1 + silence + burst2 + silence + burst3
    onsets = detect_onsets(sig, sr, min_distance_seconds=0.1)

    assert len(onsets) == 3
    # First onset near 0.0s
    assert math.isclose(onsets[0], 0.0, abs_tol=0.03)
    # Second onset near 0.25s
    assert math.isclose(onsets[1], 0.25, abs_tol=0.05)
    # Third onset near 0.50s
    assert math.isclose(onsets[2], 0.50, abs_tol=0.05)


def test_detect_onsets_return_samples() -> None:
    sr = 44100.0
    burst = _make_tone_burst(440.0, 0.2, sr)
    sample_onsets = detect_onsets(burst, sr, return_samples=True)
    assert isinstance(sample_onsets, list)
    assert len(sample_onsets) >= 1
    assert isinstance(sample_onsets[0], int)


def test_detect_onsets_silence() -> None:
    sr = 44100.0
    sig = [0.0] * 4096
    onsets = detect_onsets(sig, sr)
    assert onsets == []


def test_detect_onsets_short_signal() -> None:
    sr = 44100.0
    short_sig = [0.5, 0.8, -0.4, 0.2]
    onsets = detect_onsets(short_sig, sr)
    assert onsets == [0.0]


def test_detect_onsets_invalid_inputs() -> None:
    with pytest.raises(InvalidInputError):
        detect_onsets([], 44100.0)
    with pytest.raises(InvalidInputError):
        detect_onsets([1.0], -44100.0)
