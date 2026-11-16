"""Unit tests for audio feature extraction (MUS08, VER15)."""

from __future__ import annotations

import math

import pytest

from music_analysis import (
    rms_energy,
    spectral_centroid,
    spectral_flux,
    spectral_rolloff,
    zero_crossing_rate,
)
from music_analysis.types import BudgetExceededError, InvalidInputError


def test_spectral_centroid_single_tone() -> None:
    # 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
    # Peak at bin 2 (5000 Hz)
    spectrum = [0.0, 0.0, 1.0, 0.0, 0.0]
    c = spectral_centroid(spectrum, sample_rate_hz=20000.0)
    assert math.isclose(c, 5000.0, abs_tol=1e-3)


def test_spectral_centroid_flat_spectrum() -> None:
    # Flat spectrum centroid should be exactly at Nyquist / 2
    spectrum = [1.0] * 9
    c = spectral_centroid(spectrum, sample_rate_hz=16000.0)
    assert math.isclose(c, 4000.0, abs_tol=1e-3)


def test_spectral_centroid_zero_energy() -> None:
    spectrum = [0.0] * 5
    assert spectral_centroid(spectrum, 44100.0) == 0.0
    assert spectral_centroid([1.0], 44100.0) == 0.0


def test_spectral_flux_pair() -> None:
    spec1 = [1.0, 2.0, 3.0, 4.0]
    spec2 = [1.0, 2.0, 3.0, 4.0]
    # No change
    assert spectral_flux(spec1, spec2) == 0.0

    # Increase in upper bins
    spec3 = [1.0, 2.0, 5.0, 7.0]
    flux = spectral_flux(spec1, spec3)
    assert math.isclose(flux, 5.0, abs_tol=1e-5)

    # Decrease with rectified=True should be 0
    spec4 = [0.5, 1.0, 1.5, 2.0]
    assert spectral_flux(spec1, spec4, rectified=True) == 0.0

    # Decrease with rectified=False
    unrectified = spectral_flux(spec1, spec4, rectified=False)
    assert unrectified > 0.0


def test_spectral_flux_sequence() -> None:
    frames = [
        [1.0, 1.0],
        [2.0, 2.0],
        [1.0, 1.0],
    ]
    fluxes = spectral_flux(frames)
    assert isinstance(fluxes, list)
    assert len(fluxes) == 2
    assert math.isclose(fluxes[0], 2.0, abs_tol=1e-5)
    assert math.isclose(fluxes[1], 0.0, abs_tol=1e-5)  # rectified decrease

    assert spectral_flux([[1.0, 2.0]]) == []


def test_spectral_rolloff() -> None:
    # 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
    spectrum = [0.0, 0.0, 1.0, 0.0, 0.0]
    # All energy is at 5000 Hz
    ro = spectral_rolloff(spectrum, sample_rate_hz=20000.0, rolloff_percent=0.85)
    assert math.isclose(ro, 5000.0, abs_tol=1e-3)

    # Flat spectrum
    spectrum_flat = [1.0] * 10
    ro_flat = spectral_rolloff(
        spectrum_flat, sample_rate_hz=18000.0, rolloff_percent=0.50
    )
    assert ro_flat > 0.0 and ro_flat < 9000.0

    # Zero energy
    assert spectral_rolloff([0.0] * 5, 20000.0) == 0.0
    assert spectral_rolloff([1.0], 20000.0) == 0.0


def test_zero_crossing_rate() -> None:
    # Constant positive -> 0
    assert zero_crossing_rate([1.0, 1.0, 1.0, 1.0]) == 0.0

    # Alternating -> 1.0
    assert zero_crossing_rate([1.0, -1.0, 1.0, -1.0]) == 1.0

    # Short signal
    assert zero_crossing_rate([1.0]) == 0.0

    # Sliding window framing
    sig = [1.0, 1.0, -1.0, -1.0, 1.0, 1.0]
    zcr_framed = zero_crossing_rate(sig, frame_length=3, hop_length=1)
    assert isinstance(zcr_framed, list)
    assert len(zcr_framed) == 4

    # Short signal with framing
    short_framed = zero_crossing_rate([1.0, -1.0], frame_length=10, hop_length=5)
    assert len(short_framed) == 1


def test_rms_energy() -> None:
    # DC signal
    assert math.isclose(rms_energy([2.0, 2.0, 2.0]), 2.0, abs_tol=1e-5)

    # Empty raises InvalidInputError
    with pytest.raises(InvalidInputError):
        rms_energy([])

    # Zero signal
    assert rms_energy([0.0, 0.0]) == 0.0

    # Sine wave RMS should be A / sqrt(2)
    n = 1000
    sine = [math.sin(2.0 * math.pi * 5.0 * i / n) for i in range(n)]
    rms = rms_energy(sine)
    assert math.isclose(rms, 1.0 / math.sqrt(2.0), abs_tol=1e-2)

    # Windowed framing
    sig = [1.0] * 10
    rms_framed = rms_energy(sig, frame_length=4, hop_length=2)
    assert isinstance(rms_framed, list)
    assert len(rms_framed) == 4
    for val in rms_framed:
        assert math.isclose(val, 1.0, abs_tol=1e-5)

    # Short signal with framing
    short_framed = rms_energy([2.0, 2.0], frame_length=10, hop_length=5)
    assert len(short_framed) == 1
    assert math.isclose(short_framed[0], 2.0, abs_tol=1e-5)


def test_features_invalid_inputs() -> None:
    with pytest.raises(InvalidInputError):
        spectral_centroid([], 44100.0)
    with pytest.raises(InvalidInputError):
        spectral_centroid([1.0, 2.0], -100.0)
    with pytest.raises(BudgetExceededError):
        spectral_rolloff([1.0, 2.0], 44100.0, rolloff_percent=1.5)
    with pytest.raises(InvalidInputError):
        spectral_rolloff([1.0, 2.0], 44100.0, rolloff_percent=0.0)
    with pytest.raises(InvalidInputError):
        zero_crossing_rate([1.0], frame_length=0)
    with pytest.raises(InvalidInputError):
        rms_energy([1.0], frame_length=-1)
