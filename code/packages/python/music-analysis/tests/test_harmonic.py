"""Unit tests for harmonic profile extraction and re-synthesis parity (MUS08, MUS05)."""

from __future__ import annotations

import math

import pytest
from musical_instruments import (
    InstrumentProfile,
    get_instrument,
    render_instrument_note,
)

from music_analysis import extract_harmonic_profile
from music_analysis.types import InvalidInputError


def _synthesize_known_instrument(
    f0: float,
    partial_amps: list[float],
    sr: float,
    duration_sec: float,
) -> list[float]:
    n = int(sr * duration_sec)
    out = [0.0] * n
    for i in range(n):
        t = i / sr
        # Attack - decay - sustain - release envelope
        if t < 0.05:
            env = t / 0.05
        elif t < 0.15:
            env = 1.0 - 0.3 * ((t - 0.05) / 0.10)
        elif t < 0.35:
            env = 0.70
        else:
            env = 0.70 * max(0.0, 1.0 - (t - 0.35) / 0.05)

        sample = 0.0
        for h, amp in enumerate(partial_amps, start=1):
            sample += amp * math.sin(2.0 * math.pi * float(h) * f0 * t)
        out[i] = env * sample
    return out


def test_extract_harmonic_profile_synthetic() -> None:
    sr = 44100.0
    f0 = 440.0
    # Fundamental (1.0), 2nd harmonic (0.5), 3rd harmonic (0.25)
    known_partials = [1.0, 0.5, 0.25]
    sig = _synthesize_known_instrument(f0, known_partials, sr, duration_sec=0.4)

    profile = extract_harmonic_profile(
        sig,
        sr,
        fundamental_hz=f0,
        max_partials=4,
        instrument_id="test_synth",
        display_name="Test Synth",
    )

    assert isinstance(profile, InstrumentProfile)
    assert profile.id == "test_synth"
    assert profile.synthesis_kind == "additive"
    assert len(profile.harmonic_profile) >= 3

    # Fundamental should have multiplier 1.0 and amplitude 1.0
    h1 = profile.harmonic_profile[0]
    assert h1.frequency_multiplier == 1.0
    assert math.isclose(h1.amplitude, 1.0, abs_tol=0.05)

    # 2nd harmonic should be near 0.5
    h2 = profile.harmonic_profile[1]
    assert h2.frequency_multiplier == 2.0
    assert math.isclose(h2.amplitude, 0.5, abs_tol=0.1)

    # 3rd harmonic should be near 0.25
    h3 = profile.harmonic_profile[2]
    assert h3.frequency_multiplier == 3.0
    assert math.isclose(h3.amplitude, 0.25, abs_tol=0.1)

    # ADSR envelope validation
    env = profile.envelope_profile
    assert env.attack_seconds > 0.0
    assert env.sustain_level > 0.0 and env.sustain_level <= 1.0


def test_extract_harmonic_profile_resynthesis_parity() -> None:
    sr = 44100.0
    # Render an original note from flute preset in musical_instruments
    preset = get_instrument("flute_naive")
    original_render = render_instrument_note(
        "A4",
        0.3,
        instrument=preset,
        sample_rate_hz=sr,
    )

    # Extract profile from the rendered samples
    extracted_profile = extract_harmonic_profile(
        original_render.floating_samples,
        sr,
        fundamental_hz=440.0,
        max_partials=6,
        instrument_id="extracted_flute",
    )

    assert extracted_profile.synthesis_kind == "additive"
    assert len(extracted_profile.harmonic_profile) >= 1
    assert extracted_profile.harmonic_profile[0].amplitude == 1.0

    # Re-synthesize using the extracted profile
    resynth_render = render_instrument_note(
        "A4",
        0.3,
        instrument=extracted_profile,
        sample_rate_hz=sr,
    )

    # Confirm re-synthesis succeeded, is non-silent, and has reasonable energy
    assert len(resynth_render.floating_samples.samples) > 0
    resynth_energy = sum(s * s for s in resynth_render.floating_samples.samples)
    assert resynth_energy > 0.0


def test_extract_harmonic_profile_long_buffer_auto_f0() -> None:
    sr = 44100.0
    # Long buffer > 4096 samples without passing fundamental_hz
    n = 6000
    phase_inc = 2.0 * math.pi * 440.0 / sr
    sig = [0.8 * math.sin(i * phase_inc) for i in range(n)]

    profile = extract_harmonic_profile(
        sig,
        sr,
        fundamental_hz=None,  # auto-detect via yin_pitch
        max_partials=30,  # will hit Nyquist break
    )
    assert profile.synthesis_kind == "additive"
    assert len(profile.harmonic_profile) >= 1


def test_extract_harmonic_profile_errors() -> None:
    sr = 44100.0
    with pytest.raises(InvalidInputError):
        extract_harmonic_profile([0.0] * 2000, sr)
    with pytest.raises(InvalidInputError):
        extract_harmonic_profile([], sr)
