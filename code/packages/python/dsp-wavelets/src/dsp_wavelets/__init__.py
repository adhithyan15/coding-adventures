"""Discrete and Continuous Wavelet Transforms (DWT, 2D DWT, wavedec, SWT, CWT)."""

from __future__ import annotations

from dsp_wavelets.boundary import (
    Periodic,
    Reflect,
    Replicate,
    Symmetric,
    WaveletBoundary,
    Zero,
    parse_wavelet_boundary,
    sample_with_boundary,
)
from dsp_wavelets.cwt import cwt
from dsp_wavelets.dwt import (
    dwt,
    dwt_1d,
    dwt_2d,
    forward_level_lengths,
    idwt,
    idwt_1d,
    idwt_2d,
)
from dsp_wavelets.errors import (
    MAX_SAMPLES,
    MAX_SCALES,
    MAX_WAVELET_LEVELS,
    EmptySignalError,
    FftError,
    InvalidCoefficientsError,
    InvalidParamError,
    SignalTooShortError,
    WaveletError,
    validate_floats,
)
from dsp_wavelets.swt import iswt, swt
from dsp_wavelets.wavedec import wavedec, waverec
from dsp_wavelets.wavelet import (
    Wavelet,
    WaveletFamily,
    parse_wavelet,
)

__all__ = [
    "MAX_SAMPLES",
    "MAX_SCALES",
    "MAX_WAVELET_LEVELS",
    "EmptySignalError",
    "FftError",
    "InvalidCoefficientsError",
    "InvalidParamError",
    "Periodic",
    "Reflect",
    "Replicate",
    "SignalTooShortError",
    "Symmetric",
    "Wavelet",
    "WaveletBoundary",
    "WaveletError",
    "WaveletFamily",
    "Zero",
    "cwt",
    "dwt",
    "dwt_1d",
    "dwt_2d",
    "forward_level_lengths",
    "idwt",
    "idwt_1d",
    "idwt_2d",
    "iswt",
    "parse_wavelet",
    "parse_wavelet_boundary",
    "sample_with_boundary",
    "swt",
    "validate_floats",
    "wavedec",
    "waverec",
]
