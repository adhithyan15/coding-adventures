"""Defensive caps, validation helpers, and error types for dsp_wavelets."""

from __future__ import annotations

import math
from collections.abc import Sequence

# CWE-400 defensive bounds matching Rust and warrant specs
MAX_SAMPLES: int = 1 << 24  # 16,777,216 samples
MAX_WAVELET_LEVELS: int = 32  # Defensive pyramid level cap
MAX_SCALES: int = 4096  # Continuous wavelet transform scale cap


class WaveletError(Exception):
    """Base exception for all wavelet errors."""


class EmptySignalError(WaveletError):
    """Raised when the input signal is empty."""


class InvalidParamError(WaveletError):
    """Raised on invalid parameters (levels == 0, bounds exceeded, non-finite)."""


class SignalTooShortError(WaveletError):
    """Raised when the signal is too short to support requested decomposition levels."""


class InvalidCoefficientsError(WaveletError):
    """Raised when coefficient shapes or values are invalid."""


class FftError(WaveletError):
    """Wraps FFT failure."""


def validate_floats(
    seq: Sequence[float],
    name: str,
    exc_type: type[WaveletError] = InvalidParamError,
) -> None:
    """Validate that all items in seq are finite floats (CWE-20)."""
    for idx, val in enumerate(seq):
        if not math.isfinite(val):
            raise exc_type(
                f"non-finite float in {name} at index {idx}: {val}"
            )
