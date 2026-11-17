"""Continuous Wavelet Transform (CWT) with Morlet and Ricker wavelets."""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_wavelets.errors import (
    MAX_SAMPLES,
    MAX_SCALES,
    EmptySignalError,
    InvalidParamError,
    validate_floats,
)
from dsp_wavelets.wavelet import Wavelet, WaveletFamily, parse_wavelet


def cwt(
    signal: Sequence[float],
    scales: Sequence[float],
    wavelet: Wavelet | str = "morlet",
) -> list[float]:
    """Compute the Continuous Wavelet Transform (CWT).

    Returns a flattened row-major [len(scales), len(signal)] list of floats.
    """
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if len(scales) == 0:
        raise InvalidParamError("scales must be non-empty")
    if len(signal) > MAX_SAMPLES:
        raise InvalidParamError(f"signal length exceeds MAX_SAMPLES {MAX_SAMPLES}")
    if len(scales) > MAX_SCALES:
        raise InvalidParamError(f"scales length exceeds MAX_SCALES {MAX_SCALES}")

    total_len = len(scales) * len(signal)
    if total_len > MAX_SAMPLES:
        raise InvalidParamError("CWT output length exceeds MAX_SAMPLES")

    for idx, s in enumerate(scales):
        if not math.isfinite(s) or s <= 0.0:
            raise InvalidParamError(
                f"scales must be positive and finite; got {s} at index {idx}"
            )

    validate_floats(signal, "signal")

    w = parse_wavelet(wavelet)
    if w.family not in (WaveletFamily.MORLET, WaveletFamily.RICKER):
        raise InvalidParamError(
            f"CWT supports Morlet and Ricker wavelets only; got {w.name}"
        )

    n = len(signal)
    pi_pow_neg_quarter = math.pow(math.pi, -0.25)
    out: list[float] = [0.0] * total_len

    if w.family == WaveletFamily.MORLET:
        # psi(t) = pi^(-1/4) * e^(i omega_0 t) * e^(-t^2 / 2), omega_0 = 6.0
        omega_0 = 6.0
        out_idx = 0
        for s in scales:
            inv_sqrt_s = 1.0 / math.sqrt(s)
            inv_s = 1.0 / s
            half_support = min(int(math.ceil(8.0 * s)), n)

            for i in range(n):
                re_acc = 0.0
                im_acc = 0.0
                start_k = max(0, i - half_support)
                end_k = min(n - 1, i + half_support)

                for k in range(start_k, end_k + 1):
                    t = (k - i) * inv_s
                    envelope = pi_pow_neg_quarter * math.exp(-0.5 * t * t)
                    phase = omega_0 * t
                    re_psi = envelope * math.cos(phase)
                    im_psi = -envelope * math.sin(phase)
                    x = signal[k]
                    re_acc += x * re_psi
                    im_acc += x * im_psi

                mag = inv_sqrt_s * math.sqrt(re_acc * re_acc + im_acc * im_acc)
                out[out_idx] = mag
                out_idx += 1

    elif w.family == WaveletFamily.RICKER:
        # psi(t) = 2/sqrt(3) * pi^(-1/4) * (1 - t^2) * e^(-t^2 / 2)
        c = (2.0 / math.sqrt(3.0)) * pi_pow_neg_quarter
        out_idx = 0
        for s in scales:
            inv_sqrt_s = 1.0 / math.sqrt(s)
            inv_s = 1.0 / s
            half_support = min(int(math.ceil(8.0 * s)), n)

            for i in range(n):
                acc = 0.0
                start_k = max(0, i - half_support)
                end_k = min(n - 1, i + half_support)

                for k in range(start_k, end_k + 1):
                    t = (k - i) * inv_s
                    psi = c * (1.0 - t * t) * math.exp(-0.5 * t * t)
                    acc += signal[k] * psi

                out[out_idx] = inv_sqrt_s * acc
                out_idx += 1

    return out
