"""Mel filterbank, mel-spectrogram, and MFCC computation."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_dct import DctNorm, DctType, dct

from dsp_stft.errors import (
    MAX_FFT_LEN,
    MAX_SAMPLES,
    InvalidParamError,
)
from dsp_stft.spectrogram import power_spectrogram
from dsp_stft.windows import WindowType

EPS: float = 1e-10


def hz_to_mel(f: float) -> float:
    """Convert Hz to mel scale using the HTK convention.

    mel(f) = 2595 * log10(1 + f / 700).
    """
    if not math.isfinite(f) or f < 0.0:
        raise InvalidParamError(f"frequency must be finite and >= 0; got {f}")
    return 2595.0 * math.log10(1.0 + f / 700.0)


def mel_to_hz(m: float) -> float:
    """Convert mel scale to Hz using the inverse HTK convention.

    hz(m) = 700 * (10^(m / 2595) - 1).
    """
    if not math.isfinite(m) or m < 0.0:
        raise InvalidParamError(f"mel value must be finite and >= 0; got {m}")
    return 700.0 * (math.pow(10.0, m / 2595.0) - 1.0)


def mel_filterbank(
    n_mels: int,
    n_fft: int,
    sample_rate: float,
    f_min: float = 0.0,
    f_max: float | None = None,
) -> list[float]:
    """Build an [n_mels, n_fft // 2 + 1] triangular mel filterbank.

    Rows are normalized to sum to 1.0 (weighted average of power bins).
    Returns an empty list on degenerate arguments (n_mels <= 0, n_fft <= 0, etc.).
    """
    if n_mels <= 0 or n_fft <= 0 or n_mels > MAX_SAMPLES or n_fft > MAX_FFT_LEN:
        return []
    if not math.isfinite(sample_rate) or sample_rate <= 0.0:
        return []

    if f_max is None:
        f_max = sample_rate * 0.5
    if not math.isfinite(f_min) or not math.isfinite(f_max):
        return []
    if not (0.0 <= f_min < f_max <= sample_rate * 0.5):
        return []

    bins = n_fft // 2 + 1
    total_len = n_mels * bins
    if total_len > MAX_SAMPLES:
        return []

    mel_min = hz_to_mel(f_min)
    mel_max = hz_to_mel(f_max)
    step = (mel_max - mel_min) / float(n_mels + 1)
    bin_per_hz = float(n_fft) / sample_rate

    bin_pts = [
        mel_to_hz(mel_min + step * float(i)) * bin_per_hz
        for i in range(n_mels + 2)
    ]

    filterbank = [0.0] * total_len

    # Triangular filter rasterisation (iterative, CWE-674)
    for m in range(n_mels):
        left = bin_pts[m]
        center = bin_pts[m + 1]
        right = bin_pts[m + 2]
        row_offset = m * bins

        for k in range(bins):
            kf = float(k)
            if kf < left or kf > right:
                w = 0.0
            elif kf <= center:
                w = (kf - left) / (center - left) if center > left else 0.0
            elif right > center:
                w = (right - kf) / (right - center)
            else:
                w = 0.0
            filterbank[row_offset + k] = w

        row_sum = sum(filterbank[row_offset : row_offset + bins])
        if row_sum > 0.0:
            for k in range(bins):
                filterbank[row_offset + k] /= row_sum

    return filterbank


def mel_spectrogram(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    n_mels: int,
    sample_rate: float,
    window: WindowType | str = WindowType.HANN,
    f_min: float = 0.0,
    f_max: float | None = None,
) -> list[float]:
    """Compute mel-spectrogram: mel_filterbank @ power_spectrogram."""
    if n_mels <= 0:
        raise InvalidParamError(f"n_mels must be > 0; got {n_mels}")
    if n_mels > MAX_SAMPLES:
        raise InvalidParamError(
            f"n_mels {n_mels} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    if not math.isfinite(sample_rate) or sample_rate <= 0.0:
        raise InvalidParamError(
            f"sample_rate must be positive and finite; got {sample_rate}"
        )

    if f_max is None:
        f_max = sample_rate * 0.5
    if not math.isfinite(f_min) or not math.isfinite(f_max):
        raise InvalidParamError("f_min and f_max must be finite")
    if not (0.0 <= f_min < f_max <= sample_rate * 0.5):
        raise InvalidParamError(
            f"frequencies must satisfy 0 <= f_min < f_max <= sample_rate / 2; "
            f"got f_min={f_min}, f_max={f_max}, Nyquist={sample_rate * 0.5}"
        )

    bins = n_fft // 2 + 1
    fb_len = n_mels * bins
    if fb_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"mel filterbank size {fb_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    fb = mel_filterbank(n_mels, n_fft, sample_rate, f_min=f_min, f_max=f_max)
    if len(fb) != fb_len:
        raise InvalidParamError("mel_filterbank generation failed")

    power = power_spectrogram(signal, n_fft, hop_length, window)
    num_frames = len(power) // bins
    out_len = num_frames * n_mels
    if out_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"mel spectrogram length {out_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    out = [0.0] * out_len
    for t in range(num_frames):
        p_offset = t * bins
        out_offset = t * n_mels
        for m in range(n_mels):
            fb_offset = m * bins
            acc = 0.0
            for k in range(bins):
                acc += fb[fb_offset + k] * power[p_offset + k]
            out[out_offset + m] = acc

    return out


def mfcc(
    signal: Sequence[float],
    n_fft: int,
    hop_length: int,
    n_mels: int,
    n_mfcc: int,
    sample_rate: float,
    window: WindowType | str = WindowType.HANN,
    f_min: float = 0.0,
    f_max: float | None = None,
) -> list[float]:
    """Compute Mel-Frequency Cepstral Coefficients (MFCCs).

    mfcc = DCT-II_ortho(ln(mel_spectrogram + eps))[:, :n_mfcc].
    """
    if n_mfcc <= 0:
        raise InvalidParamError(f"n_mfcc must be > 0; got {n_mfcc}")
    if n_mfcc > n_mels:
        raise InvalidParamError(
            f"n_mfcc {n_mfcc} > n_mels {n_mels} "
            f"(cannot keep more DCT coefficients than DCT inputs)"
        )

    mel = mel_spectrogram(
        signal,
        n_fft,
        hop_length,
        n_mels,
        sample_rate,
        window=window,
        f_min=f_min,
        f_max=f_max,
    )

    num_frames = len(mel) // n_mels
    log_mel = [math.log(v + EPS) for v in mel]

    total_mfcc_len = num_frames * n_mfcc
    if total_mfcc_len > MAX_SAMPLES:
        raise InvalidParamError(
            f"MFCC output length {total_mfcc_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    out = [0.0] * total_mfcc_len
    for t in range(num_frames):
        frame_slice = log_mel[t * n_mels : (t + 1) * n_mels]
        coeffs = dct(frame_slice, DctType.II, DctNorm.ORTHO)
        out_offset = t * n_mfcc
        for c in range(n_mfcc):
            out[out_offset + c] = coeffs[c]

    return out
