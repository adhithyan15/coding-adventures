"""Music Understanding, Analysis & Sample-to-Score Transcription (MUS08)."""

from __future__ import annotations

from music_analysis.features import (
    rms_energy,
    spectral_centroid,
    spectral_flux,
    spectral_rolloff,
    zero_crossing_rate,
)
from music_analysis.frequency import frequency_to_note
from music_analysis.harmonic import extract_harmonic_profile
from music_analysis.onset import detect_onsets
from music_analysis.pitch import hps_pitch, yin_pitch, yin_pitch_track
from music_analysis.transcription import transcribe_pcm_to_portable_score
from music_analysis.types import (
    MAX_BPM,
    MAX_FFT_LEN,
    MAX_FRAMES,
    MAX_ONSETS,
    MAX_PARTIALS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MIN_BPM,
    MIN_SAMPLE_RATE_HZ,
    BudgetExceededError,
    InvalidInputError,
    MusicAnalysisError,
    NotePitch,
)

__version__ = "0.1.0"

__all__ = [
    "MAX_BPM",
    "MAX_FFT_LEN",
    "MAX_FRAMES",
    "MAX_ONSETS",
    "MAX_PARTIALS",
    "MAX_SAMPLE_RATE_HZ",
    "MAX_SAMPLES",
    "MIN_BPM",
    "MIN_SAMPLE_RATE_HZ",
    "BudgetExceededError",
    "InvalidInputError",
    "MusicAnalysisError",
    "NotePitch",
    "__version__",
    "detect_onsets",
    "extract_harmonic_profile",
    "frequency_to_note",
    "hps_pitch",
    "rms_energy",
    "spectral_centroid",
    "spectral_flux",
    "spectral_rolloff",
    "transcribe_pcm_to_portable_score",
    "yin_pitch",
    "yin_pitch_track",
    "zero_crossing_rate",
]
