"""Stationary Wavelet Transform (SWT / undecimated DWT) and inverse."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_wavelets.boundary import (
    WaveletBoundary,
    parse_wavelet_boundary,
    sample_with_boundary,
)
from dsp_wavelets.errors import (
    MAX_SAMPLES,
    MAX_WAVELET_LEVELS,
    EmptySignalError,
    InvalidCoefficientsError,
    InvalidParamError,
    validate_floats,
)
from dsp_wavelets.wavelet import FRAC_1_SQRT_2, Wavelet, WaveletFamily, parse_wavelet


def _require_orthogonal_swt(w: Wavelet) -> None:
    if w.family not in (
        WaveletFamily.HAAR,
        WaveletFamily.DAUBECHIES,
        WaveletFamily.SYMLETS,
        WaveletFamily.COIFLETS,
    ):
        raise InvalidParamError(
            f"SWT supports orthogonal wavelets only; got {w.name}"
        )


def swt(
    signal: Sequence[float],
    wavelet: Wavelet | str = "haar",
    level: int = 1,
    boundary: WaveletBoundary | str = WaveletBoundary.PERIODIC,
) -> list[tuple[list[float], list[float]]]:
    """Compute stationary (undecimated) wavelet transform.

    Returns a list of (cA, cD) tuples from level J down to 1.
    Each array has the same length as the input signal.
    """
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if level <= 0:
        raise InvalidParamError(f"level must be > 0; got {level}")
    if level > MAX_WAVELET_LEVELS:
        raise InvalidParamError(f"level {level} exceeds MAX_WAVELET_LEVELS")
    if len(signal) > MAX_SAMPLES:
        raise InvalidParamError("signal length exceeds MAX_SAMPLES")
    validate_floats(signal, "signal")

    n = len(signal)
    w = parse_wavelet(wavelet)
    _require_orthogonal_swt(w)
    b = parse_wavelet_boundary(boundary)

    # Scaled filters for undecimated transform
    h_scaled = [tap * FRAC_1_SQRT_2 for tap in w.dec_lo]
    g_scaled = [tap * FRAC_1_SQRT_2 for tap in w.dec_hi]
    filter_len = len(h_scaled)

    current = list(signal)
    coeffs: list[tuple[list[float], list[float]]] = []

    for j in range(1, level + 1):
        step = 1 << (j - 1)
        ca = [0.0] * n
        cd = [0.0] * n
        for idx in range(n):
            acc_h = 0.0
            acc_g = 0.0
            for i in range(filter_len):
                src_idx = idx - i * step
                s = sample_with_boundary(current, src_idx, b)
                acc_h += h_scaled[i] * s
                acc_g += g_scaled[i] * s
            ca[idx] = acc_h
            cd[idx] = acc_g
        coeffs.append((ca, cd))
        current = ca

    # Return from coarsest (level) to finest (1)
    return list(reversed(coeffs))


def iswt(
    coeffs: Sequence[tuple[Sequence[float], Sequence[float]]],
    wavelet: Wavelet | str = "haar",
    boundary: WaveletBoundary | str = WaveletBoundary.PERIODIC,
) -> list[float]:
    """Inverse stationary wavelet transform."""
    if len(coeffs) == 0:
        raise EmptySignalError("coeffs must be non-empty")

    level = len(coeffs)
    if level > MAX_WAVELET_LEVELS:
        raise InvalidParamError(f"level {level} exceeds MAX_WAVELET_LEVELS")

    w = parse_wavelet(wavelet)
    _require_orthogonal_swt(w)
    b = parse_wavelet_boundary(boundary)

    # First tuple is (cA_J, cD_J)
    ca_top, cd_top = coeffs[0]
    n = len(ca_top)
    if n <= 0:
        raise EmptySignalError("subbands must be non-empty")
    if n > MAX_SAMPLES:
        raise InvalidParamError(f"subband length {n} exceeds MAX_SAMPLES {MAX_SAMPLES}")

    # Validate all subbands shapes and finite floats BEFORE allocating rec
    for j_idx, (ca_j, cd_j) in enumerate(coeffs):
        if len(ca_j) != n or len(cd_j) != n:
            raise InvalidCoefficientsError("all subbands must have length N")
        validate_floats(ca_j, f"cA_{j_idx}", exc_type=InvalidCoefficientsError)
        validate_floats(cd_j, f"cD_{j_idx}", exc_type=InvalidCoefficientsError)

    h_scaled = [tap * FRAC_1_SQRT_2 for tap in w.rec_lo]
    g_scaled = [tap * FRAC_1_SQRT_2 for tap in w.rec_hi]
    filter_len_h = len(h_scaled)
    filter_len_g = len(g_scaled)

    current_ca = list(ca_top)

    for j_idx, (_ca_j, cd_j) in enumerate(coeffs):
        j = level - j_idx  # Level from J down to 1
        step = 1 << (j - 1)

        rec = [0.0] * n
        for idx in range(n):
            acc = 0.0
            for i in range(filter_len_h):
                src_idx = idx + i * step
                ca_val = sample_with_boundary(current_ca, src_idx, b)
                acc += h_scaled[i] * ca_val
            for i in range(filter_len_g):
                src_idx = idx + i * step
                cd_val = sample_with_boundary(cd_j, src_idx, b)
                acc += g_scaled[i] * cd_val
            rec[idx] = acc
        current_ca = rec

    return current_ca
