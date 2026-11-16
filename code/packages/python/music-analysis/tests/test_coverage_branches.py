"""Targeted tests for branch coverage in music analysis (VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence

import pytest
from musical_instruments import get_instrument, render_instrument_note

from music_analysis import (
    MAX_FRAMES,
    BudgetExceededError,
    InvalidInputError,
    extract_harmonic_profile,
    hps_pitch,
    rms_energy,
    spectral_rolloff,
    transcribe_pcm_to_portable_score,
    yin_pitch_track,
    zero_crossing_rate,
)


class _VirtualLargeSequence(Sequence[float]):
    def __init__(self, virtual_length: int) -> None:
        self._len = virtual_length

    def __len__(self) -> int:
        return self._len

    def __getitem__(self, index: int) -> float:  # type: ignore[override]
        if index < 0 or index >= self._len:
            raise IndexError("index out of range")
        return 0.1


def test_coverage_spectral_rolloff_full() -> None:
    # 1.0 rolloff percentage reaches end of loop without accumulator exceeding
    sr = 44100.0
    res = spectral_rolloff([1.0, 2.0, 3.0], sr, rolloff_percent=1.0)
    assert math.isclose(res, sr / 2.0, abs_tol=1e-3)


def test_coverage_frames_budget_exceeded() -> None:
    too_many = _VirtualLargeSequence(MAX_FRAMES + 2)
    with pytest.raises(BudgetExceededError):
        zero_crossing_rate(too_many, frame_length=1, hop_length=1)
    with pytest.raises(BudgetExceededError):
        rms_energy(too_many, frame_length=1, hop_length=1)
    with pytest.raises(BudgetExceededError):
        yin_pitch_track(too_many, 44100.0, frame_length=1, hop_length=1)


def test_coverage_harmonic_sample_buffer_and_nyquist_break() -> None:
    sr = 44100.0
    # Test passing SampleBuffer directly from render_instrument_note
    inst = get_instrument("violin_naive")
    rendered = render_instrument_note("A5", 0.2, instrument=inst, sample_rate_hz=sr)
    # Pass rendered.floating_samples directly (SampleBuffer instance)
    profile = extract_harmonic_profile(
        rendered.floating_samples,
        sr,
        fundamental_hz=880.0,
        max_partials=50,  # 50 * 880 = 44000 > 22050 Nyquist, triggers break
    )
    assert profile.synthesis_kind == "additive"


def test_coverage_harmonic_pitch_fail_noise(monkeypatch: pytest.MonkeyPatch) -> None:
    sr = 44100.0
    # Silent signal triggers silent error
    silence = [0.0] * 1000
    with pytest.raises(InvalidInputError, match="signal is silent"):
        extract_harmonic_profile(silence, sr)

    # Signal where pitch detection fails triggers fundamental pitch error
    monkeypatch.setattr("music_analysis.harmonic.yin_pitch", lambda *a, **kw: 0.0)
    monkeypatch.setattr("music_analysis.harmonic.hps_pitch", lambda *a, **kw: 0.0)
    with pytest.raises(InvalidInputError, match="failed to detect fundamental pitch"):
        extract_harmonic_profile([0.5] * 1000, sr)


def test_coverage_transcription_branches(monkeypatch: pytest.MonkeyPatch) -> None:
    sr = 44100.0
    # Test empty sample list
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([], sample_rate_hz=sr)

    # Test signal with low energy segments
    silence = [1e-6] * 2000
    score = transcribe_pcm_to_portable_score(silence, sample_rate_hz=sr, bpm=120.0)
    assert len(score.events) == 0

    # Test signal with unpitched segment where pitch detector returns 0.0
    monkeypatch.setattr("music_analysis.transcription.yin_pitch", lambda *a, **kw: 0.0)
    monkeypatch.setattr("music_analysis.transcription.hps_pitch", lambda *a, **kw: 0.0)
    score_unpitched = transcribe_pcm_to_portable_score(
        [0.5] * 2000, sample_rate_hz=sr, bpm=120.0
    )
    assert len(score_unpitched.events) == 0


def test_coverage_hps_bounds() -> None:
    sr = 44100.0
    # Frequency bounds where k_max <= k_min
    sig = [math.sin(i * 0.1) for i in range(500)]
    assert hps_pitch(sig, sr, f_min=100.0, f_max=100.1, n_fft=512) == 0.0
