"""Magnitude, power, and log spectrograms."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_stft.errors import InvalidParamError
from dsp_stft.stft import stft
from dsp_stft.windows import WindowType


def power_spectrogram(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    window: WindowType | str = WindowType.HANN,
) -> list[float]:
    """Compute the power spectrogram |STFT|^2 for signal.

    Returns a flattened row-major [num_frames, n_fft // 2 + 1] list of floats.
    """
    complex_spec = stft(signal, n_fft, hop_length, window)
    bins = n_fft // 2 + 1
    frame_floats = bins * 2
    num_frames = len(complex_spec) // frame_floats

    out = [0.0] * (num_frames * bins)
    for m in range(num_frames):
        in_offset = m * frame_floats
        out_offset = m * bins
        for k in range(bins):
            re = complex_spec[in_offset + 2 * k]
            im = complex_spec[in_offset + 2 * k + 1]
            out[out_offset + k] = re * re + im * im
    return out


def spectrogram(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    window: WindowType | str = WindowType.HANN,
) -> list[float]:
    """Compute the power spectrogram |STFT|^2 (matching Rust dsp_stft::spectrogram)."""
    return power_spectrogram(signal, n_fft, hop_length, window)


def log_spectrogram(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    window: WindowType | str = WindowType.HANN,
    eps: float = 1e-10,
) -> list[float]:
    """Compute log-power spectrogram: ln(|STFT|^2 + eps)."""
    if not math.isfinite(eps) or eps <= 0.0:
        raise InvalidParamError(f"eps must be positive and finite; got {eps}")
    power = power_spectrogram(signal, n_fft, hop_length, window)
    out = [0.0] * len(power)
    for i, val in enumerate(power):
        out[i] = math.log(val + eps)
    return out
