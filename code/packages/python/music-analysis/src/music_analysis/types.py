"""Types, constants, exceptions, and validation for music analysis (VER15)."""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import NamedTuple

# Pre-flight CWE-400 resource bounds
MAX_SAMPLES: int = 16_777_216
MAX_FFT_LEN: int = 1_048_576
MAX_FRAMES: int = 1_048_576
MAX_PARTIALS: int = 64
MAX_ONSETS: int = 65_536
MAX_BPM: float = 1000.0
MIN_BPM: float = 1.0
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MIN_SAMPLE_RATE_HZ: float = 1000.0


class MusicAnalysisError(ValueError):
    """Base exception for all music analysis errors."""


class InvalidInputError(MusicAnalysisError):
    """Raised when an input parameter is invalid, non-finite, or malformed."""


class BudgetExceededError(MusicAnalysisError):
    """Raised when an input exceeds pre-flight resource allocation bounds."""


class NotePitch(NamedTuple):
    """Represents the result of frequency to note conversion."""

    note: str
    midi: int
    cents: float


def validate_finite_float(val: float, name: str) -> float:
    """Validate that val is a finite float (CWE-20 fail-closed)."""
    if isinstance(val, bool) or not isinstance(val, (int, float)):
        t_name = type(val).__name__
        raise InvalidInputError(f"{name} must be a real number, got {t_name}")
    float_val = float(val)
    if not math.isfinite(float_val):
        raise InvalidInputError(f"{name} must be finite; got {float_val}")
    return float_val


def validate_positive_float(
    val: float,
    name: str,
    max_val: float | None = None,
) -> float:
    """Validate that val is a finite positive float <= max_val."""
    f_val = validate_finite_float(val, name)
    if f_val <= 0.0:
        raise InvalidInputError(f"{name} must be positive (> 0.0); got {f_val}")
    if max_val is not None and f_val > max_val:
        raise BudgetExceededError(f"{name} {f_val} exceeds maximum allowed {max_val}")
    return f_val


def validate_samples(
    samples: Sequence[float],
    name: str = "samples",
) -> list[float]:
    """Validate and convert sample buffer before processing (CWE-400, CWE-20)."""
    n = len(samples)
    if n == 0:
        raise InvalidInputError(f"{name} must be non-empty")
    if n > MAX_SAMPLES:
        raise BudgetExceededError(
            f"{name} length {n} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )

    out: list[float] = [0.0] * n
    for i in range(n):
        s = samples[i]
        if isinstance(s, bool) or not isinstance(s, (int, float)):
            raise InvalidInputError(
                f"{name}[{i}] must be a float or int, got {type(s).__name__}"
            )
        fs = float(s)
        if not math.isfinite(fs):
            raise InvalidInputError(f"{name}[{i}] must be finite; got {fs}")
        out[i] = fs
    return out


def validate_spectrum(
    spectrum: Sequence[float],
    name: str = "spectrum",
) -> list[float]:
    """Validate frequency-domain magnitude or power spectrum (CWE-400, CWE-20)."""
    n = len(spectrum)
    if n == 0:
        raise InvalidInputError(f"{name} must be non-empty")
    if n > MAX_FFT_LEN:
        raise BudgetExceededError(
            f"{name} length {n} exceeds MAX_FFT_LEN={MAX_FFT_LEN}"
        )

    out: list[float] = [0.0] * n
    for i in range(n):
        s = spectrum[i]
        if isinstance(s, bool) or not isinstance(s, (int, float)):
            raise InvalidInputError(
                f"{name}[{i}] must be a float or int, got {type(s).__name__}"
            )
        fs = float(s)
        if not math.isfinite(fs):
            raise InvalidInputError(f"{name}[{i}] must be finite; got {fs}")
        out[i] = fs
    return out
