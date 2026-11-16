"""Hostile input testing for CWE-400 and CWE-20 bounds (VER15)."""

from __future__ import annotations

from collections.abc import Sequence

import pytest

from music_analysis import (
    MAX_BPM,
    MAX_FFT_LEN,
    MAX_PARTIALS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    detect_onsets,
    extract_harmonic_profile,
    frequency_to_note,
    hps_pitch,
    rms_energy,
    spectral_centroid,
    spectral_flux,
    spectral_rolloff,
    transcribe_pcm_to_portable_score,
    yin_pitch,
    zero_crossing_rate,
)
from music_analysis.types import BudgetExceededError, InvalidInputError


class _VirtualLargeSequence(Sequence[float]):
    """Sequence that reports a virtual length exceeding bounds without RAM."""

    def __init__(self, virtual_length: int, val: float = 0.5) -> None:
        self._len = virtual_length
        self._val = val

    def __len__(self) -> int:
        return self._len

    def __getitem__(self, index: int) -> float:  # type: ignore[override]
        if index < 0 or index >= self._len:
            raise IndexError("index out of range")
        return self._val


def test_hostile_max_frames_bound() -> None:
    from music_analysis.types import MAX_FRAMES
    too_many_frames = _VirtualLargeSequence(MAX_FRAMES + 1)
    with pytest.raises(BudgetExceededError):
        spectral_flux(too_many_frames)  # type: ignore[arg-type]


def test_hostile_max_samples_bound() -> None:
    too_large = _VirtualLargeSequence(MAX_SAMPLES + 1)
    with pytest.raises(BudgetExceededError):
        zero_crossing_rate(too_large)
    with pytest.raises(BudgetExceededError):
        rms_energy(too_large)
    with pytest.raises(BudgetExceededError):
        yin_pitch(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        hps_pitch(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        detect_onsets(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        extract_harmonic_profile(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        transcribe_pcm_to_portable_score(too_large, sample_rate_hz=44100.0)


def test_hostile_max_fft_len_bound() -> None:
    too_large = _VirtualLargeSequence(MAX_FFT_LEN + 1)
    with pytest.raises(BudgetExceededError):
        spectral_centroid(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        spectral_rolloff(too_large, 44100.0)
    with pytest.raises(BudgetExceededError):
        hps_pitch([1.0] * 100, 44100.0, n_fft=MAX_FFT_LEN + 1)
    with pytest.raises(BudgetExceededError):
        detect_onsets([1.0] * 100, 44100.0, n_fft=MAX_FFT_LEN + 1)


def test_hostile_max_partials_bound() -> None:
    with pytest.raises(BudgetExceededError):
        extract_harmonic_profile([1.0] * 500, 44100.0, max_partials=MAX_PARTIALS + 1)


def test_hostile_max_bpm_bound() -> None:
    with pytest.raises(BudgetExceededError):
        transcribe_pcm_to_portable_score(
            [1.0] * 500, sample_rate_hz=44100.0, bpm=MAX_BPM + 1.0
        )


def test_hostile_max_sample_rate_bound() -> None:
    with pytest.raises(BudgetExceededError):
        frequency_to_note(440.0, a4_hz=MAX_SAMPLE_RATE_HZ + 10.0)
    with pytest.raises(BudgetExceededError):
        spectral_centroid([1.0, 2.0], sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
    with pytest.raises(BudgetExceededError):
        yin_pitch([1.0] * 100, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
    with pytest.raises(BudgetExceededError):
        hps_pitch([1.0] * 100, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
    with pytest.raises(BudgetExceededError):
        detect_onsets([1.0] * 100, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
    with pytest.raises(BudgetExceededError):
        extract_harmonic_profile([1.0] * 100, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0)
    with pytest.raises(BudgetExceededError):
        transcribe_pcm_to_portable_score(
            [1.0] * 100, sample_rate_hz=MAX_SAMPLE_RATE_HZ + 1.0
        )


def test_hostile_cwe_20_nan_rejection() -> None:
    nan = float("nan")
    with pytest.raises(InvalidInputError):
        frequency_to_note(nan)
    with pytest.raises(InvalidInputError):
        frequency_to_note(440.0, a4_hz=nan)
    with pytest.raises(InvalidInputError):
        spectral_centroid([nan, 1.0])
    with pytest.raises(InvalidInputError):
        spectral_flux([nan], [1.0])
    with pytest.raises(InvalidInputError):
        spectral_rolloff([nan, 1.0])
    with pytest.raises(InvalidInputError):
        zero_crossing_rate([1.0, nan, 2.0])
    with pytest.raises(InvalidInputError):
        rms_energy([1.0, nan, 2.0])
    with pytest.raises(InvalidInputError):
        yin_pitch([nan] * 100, 44100.0)
    with pytest.raises(InvalidInputError):
        hps_pitch([nan] * 100, 44100.0)
    with pytest.raises(InvalidInputError):
        detect_onsets([nan] * 100, 44100.0)
    with pytest.raises(InvalidInputError):
        extract_harmonic_profile([nan] * 100, 44100.0)
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([nan] * 100, sample_rate_hz=44100.0)


def test_hostile_cwe_20_infinity_rejection() -> None:
    inf = float("inf")
    ninf = float("-inf")
    for bad in [inf, ninf]:
        with pytest.raises(InvalidInputError):
            frequency_to_note(bad)
        with pytest.raises(InvalidInputError):
            spectral_centroid([1.0, bad])
        with pytest.raises(InvalidInputError):
            zero_crossing_rate([1.0, bad])
        with pytest.raises(InvalidInputError):
            rms_energy([1.0, bad])
        with pytest.raises(InvalidInputError):
            yin_pitch([1.0, bad], 44100.0)


def test_hostile_boolean_type_rejection() -> None:
    with pytest.raises(InvalidInputError):
        frequency_to_note(True)  # type: ignore[arg-type]
    with pytest.raises(InvalidInputError):
        spectral_centroid([True, False])  # type: ignore[list-item]
    with pytest.raises(InvalidInputError):
        zero_crossing_rate([True, False])  # type: ignore[list-item]
    with pytest.raises(InvalidInputError):
        rms_energy([True, False])  # type: ignore[list-item]


def test_hostile_security_reviewer_findings() -> None:
    from pcm_audio import PCMBuffer, PCMFormat

    # Finding 1: sample_rate_hz rejection of bool/string
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([0.5] * 100, sample_rate_hz=True)  # type: ignore[arg-type]
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score([0.5] * 100, sample_rate_hz="44100")  # type: ignore[arg-type]

    # Finding 2: hop_length=0, 0.0, or False fail closed
    for bad_hop in [0, 0.0, False]:
        with pytest.raises(InvalidInputError):
            zero_crossing_rate([0.5] * 100, frame_length=32, hop_length=bad_hop)  # type: ignore[arg-type]
        with pytest.raises(InvalidInputError):
            rms_energy([0.5] * 100, frame_length=32, hop_length=bad_hop)  # type: ignore[arg-type]

    # Finding 3: spectral_flux rejects n_frames == 0, and validates n_frames == 1
    with pytest.raises(InvalidInputError):
        spectral_flux([])  # type: ignore[arg-type]
    with pytest.raises(InvalidInputError):
        spectral_flux([[float("nan"), 1.0]])
    # Valid single frame returns empty list
    assert spectral_flux([[1.0, 2.0, 3.0]]) == []

    # Finding 4: hps_pitch num_harmonics validation
    for bad_h in [0, -1, False, True, "5"]:
        with pytest.raises(InvalidInputError):
            hps_pitch([0.5] * 200, 44100.0, num_harmonics=bad_h)  # type: ignore[arg-type]
    with pytest.raises(BudgetExceededError):
        hps_pitch([0.5] * 200, 44100.0, num_harmonics=MAX_PARTIALS + 1)

    # Finding 5: ppq, quantize_grid_beats, bpm, and min sample rate upfront validation
    for bad_ppq in [0, -1, True, False, 9601, "480"]:
        with pytest.raises(InvalidInputError):
            transcribe_pcm_to_portable_score(
                [0.5] * 100, sample_rate_hz=44100.0, ppq=bad_ppq  # type: ignore[arg-type]
            )
    for bad_q in [0, 0.0, -0.25, False, True, "0.25"]:
        with pytest.raises(InvalidInputError):
            transcribe_pcm_to_portable_score(
                [0.5] * 100, sample_rate_hz=44100.0, quantize_grid_beats=bad_q  # type: ignore[arg-type]
            )
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score(
            [0.5] * 100, sample_rate_hz=44100.0, bpm=0.5  # below MIN_BPM=1.0
        )
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score(
            [0.5] * 100, sample_rate_hz=500.0  # below MIN_SAMPLE_RATE_HZ=1000.0
        )

    # Finding 6: CWE-400 pre-flight bounds for onset and PCMBuffer
    too_many_frames_sig = _VirtualLargeSequence(2_000_000, val=0.5)
    with pytest.raises(BudgetExceededError):
        detect_onsets(too_many_frames_sig, 44100.0, hop_length=1, n_fft=1024)

    # Empty PCMBuffer fails closed
    empty_buf = PCMBuffer(samples=(), pcm_format=PCMFormat(sample_rate_hz=44100.0))
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score(empty_buf)

    # Low sample rate PCMBuffer fails closed
    low_sr_buf = PCMBuffer(samples=(500,), pcm_format=PCMFormat(sample_rate_hz=500.0))
    with pytest.raises(InvalidInputError):
        transcribe_pcm_to_portable_score(low_sr_buf)

    # Giant PCMBuffer exceeds MAX_SAMPLES pre-flight in transcribe_pcm_to_portable_score
    large_pcm_seq = _VirtualLargeSequence(MAX_SAMPLES + 1)
    large_buf = object.__new__(PCMBuffer)
    object.__setattr__(large_buf, "pcm_format", PCMFormat(sample_rate_hz=44100.0))
    object.__setattr__(large_buf, "samples", large_pcm_seq)
    with pytest.raises(BudgetExceededError):
        transcribe_pcm_to_portable_score(large_buf)
