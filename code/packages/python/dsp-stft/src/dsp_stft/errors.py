"""Defensive constants, validation helpers, and error types for dsp_stft."""

from __future__ import annotations

import math
from collections.abc import Sequence

# CWE-400 defensive bounds matching Rust and warrant specs
MAX_FFT_LEN: int = 1 << 20  # 1,048,576
MAX_SAMPLES: int = 1 << 24  # 16,777,216 samples
MAX_FRAMES: int = 1 << 20  # 1,048,576 frames
MAX_MELS: int = 1 << 24


class StftError(Exception):
    """Base exception for all STFT errors."""


class EmptySignalError(StftError):
    """Raised when the input signal is empty."""


class InvalidParamError(StftError):
    """Raised on invalid parameters (e.g. non-positive n_fft or hop, non-finite)."""


class SignalTooShortError(StftError):
    """Raised when the signal length is less than n_fft in strict mode."""


class InvalidSpectrogramError(StftError):
    """Raised when the spectrogram buffer shape or contents are invalid."""


class FftError(StftError):
    """Wraps FFT failure."""


def validate_floats(
    seq: Sequence[float], name: str, exc_type: type[StftError] = InvalidParamError
) -> None:
    """Validate that all items in seq are finite floats (CWE-20)."""
    for idx, val in enumerate(seq):
        if not math.isfinite(val):
            raise exc_type(
                f"non-finite float in {name} at index {idx}: {val}"
            )
