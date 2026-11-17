"""Multi-level 1-D wavelet decomposition and reconstruction."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_wavelets.boundary import WaveletBoundary, parse_wavelet_boundary
from dsp_wavelets.dwt import _filter_and_downsample, _synthesize_one_level
from dsp_wavelets.errors import (
    MAX_SAMPLES,
    MAX_WAVELET_LEVELS,
    EmptySignalError,
    InvalidCoefficientsError,
    InvalidParamError,
    SignalTooShortError,
    validate_floats,
)
from dsp_wavelets.wavelet import Wavelet, WaveletFamily, parse_wavelet


def _require_orthogonal_wavedec(w: Wavelet) -> None:
    if w.family not in (
        WaveletFamily.HAAR,
        WaveletFamily.DAUBECHIES,
        WaveletFamily.SYMLETS,
        WaveletFamily.COIFLETS,
    ):
        raise InvalidParamError(
            f"wavedec supports orthogonal wavelets only; got {w.name}"
        )


def wavedec(
    signal: Sequence[float],
    wavelet: Wavelet | str = "haar",
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
    level: int | None = None,
) -> list[list[float]]:
    """Compute multi-level 1-D wavelet decomposition.

    Returns [cA_J, cD_J, cD_{J-1}, ..., cD_1].
    """
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if len(signal) > MAX_SAMPLES:
        raise InvalidParamError("signal length exceeds MAX_SAMPLES")
    validate_floats(signal, "signal")

    w = parse_wavelet(wavelet)
    _require_orthogonal_wavedec(w)
    b = parse_wavelet_boundary(boundary)

    if level is None:
        # Default level = floor(log2(len(signal) / max(1, filter_len - 1)))
        max_level = max(1, (len(signal)).bit_length() - 1)
        levels = min(max_level, MAX_WAVELET_LEVELS)
    else:
        if level <= 0:
            raise InvalidParamError(f"level must be > 0; got {level}")
        if level > MAX_WAVELET_LEVELS:
            raise InvalidParamError(f"level {level} exceeds MAX_WAVELET_LEVELS")
        levels = level

    if len(signal) < (1 << (levels - 1)):
        raise SignalTooShortError(
            f"signal length {len(signal)} too short for {levels} levels"
        )

    current = list(signal)
    details: list[list[float]] = []

    for _ in range(levels):
        ca, cd = _filter_and_downsample(current, w.dec_lo, w.dec_hi, b)
        details.append(cd)
        current = ca

    result: list[list[float]] = [current]
    for cd in reversed(details):
        result.append(cd)
    return result


def waverec(
    coeffs: Sequence[Sequence[float]],
    wavelet: Wavelet | str = "haar",
    boundary: WaveletBoundary | str = WaveletBoundary.SYMMETRIC,
) -> list[float]:
    """Reconstruct 1-D signal from [cA_J, cD_J, ..., cD_1] coefficients."""
    if len(coeffs) < 2:
        raise InvalidCoefficientsError(
            "coeffs list must contain at least [cA, cD] bands"
        )
    levels = len(coeffs) - 1
    if levels > MAX_WAVELET_LEVELS:
        raise InvalidParamError(
            f"number of levels ({levels}) exceeds "
            f"MAX_WAVELET_LEVELS {MAX_WAVELET_LEVELS}"
        )

    for idx, band in enumerate(coeffs):
        if len(band) == 0:
            raise EmptySignalError(f"band {idx} in coeffs is empty")
        if len(band) > MAX_SAMPLES:
            raise InvalidParamError(
                f"band {idx} length {len(band)} exceeds MAX_SAMPLES {MAX_SAMPLES}"
            )
        validate_floats(band, f"band_{idx}", exc_type=InvalidCoefficientsError)

    w = parse_wavelet(wavelet)
    _require_orthogonal_wavedec(w)
    b = parse_wavelet_boundary(boundary)

    current = list(coeffs[0])
    # Successively synthesize each detail level
    for j in range(1, len(coeffs)):
        cd = coeffs[j]
        # Target length is approximately 2 * len(current)
        target_len = 2 * len(current)
        if target_len > MAX_SAMPLES:
            raise InvalidParamError("target_len exceeds MAX_SAMPLES")
        current = _synthesize_one_level(current, cd, w.rec_lo, w.rec_hi, target_len, b)

    return current
