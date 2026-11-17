"""FIR (Finite Impulse Response) filtering via direct and FFT convolution.

Conforms to DSP03 Phases 2-3 and VER15 security/weakness matrix invariants:
- Direct linear convolution with mode='full' (length N + K - 1).
- FFT-accelerated convolution (fir_fft) via dsp_fft.rfft_scalar.
- Pre-flight bounds checks on MAX_SAMPLES and MAX_TAPS before allocating.
- CWE-20 finite float validation fail-closed.
- CWE-674 strictly iterative execution (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_fft import irfft_scalar, rfft_scalar

# Maximum signal length in samples (16,777,216 samples = 64 MB of float32).
MAX_SAMPLES: int = 1 << 24

# Maximum filter taps (1,048,576 taps).
MAX_TAPS: int = 1 << 20

# Maximum IIR filter order.
MAX_ORDER: int = 1024


class FilterError(ValueError):
    """Base exception for filter primitives."""


class EmptySignalError(FilterError):
    """Raised when the input signal is empty."""


class EmptyKernelError(FilterError):
    """Raised when a filter kernel or coefficient list is empty."""


class InvalidCoefficientError(FilterError):
    """Raised on non-finite floats, zero a[0], or bounds violations."""


class FftError(FilterError):
    """Raised on underlying FFT transform errors."""


def _validate_fir_inputs(
    signal: Sequence[float], kernel: Sequence[float]
) -> tuple[int, int, int]:
    """Validate FIR signal and kernel bounds, emptiness, and finiteness.

    Returns (n, k, out_len).
    """
    n = len(signal)
    if n == 0:
        raise EmptySignalError("filter signal must be non-empty")
    k = len(kernel)
    if k == 0:
        raise EmptyKernelError("filter kernel must be non-empty")

    if n > MAX_SAMPLES:
        raise InvalidCoefficientError(
            f"signal length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )
    if k > MAX_TAPS:
        raise InvalidCoefficientError(
            f"kernel length {k} exceeds MAX_TAPS ({MAX_TAPS})"
        )

    out_len = n + k - 1
    if out_len > MAX_SAMPLES:
        raise InvalidCoefficientError(
            f"convolution output length {out_len} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    # Fail-closed finite float validation
    for x in signal:
        if not math.isfinite(float(x)):
            raise InvalidCoefficientError("non-finite float in signal")
    for h in kernel:
        if not math.isfinite(float(h)):
            raise InvalidCoefficientError("non-finite float in kernel")

    return n, k, out_len


def fir(signal: Sequence[float], kernel: Sequence[float]) -> list[float]:
    """Compute FIR filtering via direct linear convolution (mode='full').

    Output length is len(signal) + len(kernel) - 1.
    """
    n, k, out_len = _validate_fir_inputs(signal, kernel)

    out = [0.0] * out_len
    for i in range(out_len):
        j_lo = max(0, i + 1 - n)
        j_hi = min(k, i + 1)
        acc = 0.0
        for j in range(j_lo, j_hi):
            acc += float(kernel[j]) * float(signal[i - j])
        out[i] = acc

    return out


def fir_fft(signal: Sequence[float], kernel: Sequence[float]) -> list[float]:
    """Compute FIR filtering via FFT-accelerated fast convolution.

    Returns the exact linear convolution matching fir(signal, kernel).
    """
    n, k, out_len = _validate_fir_inputs(signal, kernel)

    fft_len = max(2, 1 << (out_len - 1).bit_length())
    if fft_len > MAX_SAMPLES:
        raise InvalidCoefficientError(
            f"FFT length {fft_len} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    sig_pad = [float(x) for x in signal] + [0.0] * (fft_len - n)
    ker_pad = [float(x) for x in kernel] + [0.0] * (fft_len - k)

    sig_spec = rfft_scalar(sig_pad)
    ker_spec = rfft_scalar(ker_pad)

    bins = fft_len // 2 + 1
    prod = [0.0] * (2 * bins)
    for b in range(bins):
        r1 = sig_spec[2 * b]
        i1 = sig_spec[2 * b + 1]
        r2 = ker_spec[2 * b]
        i2 = ker_spec[2 * b + 1]
        prod[2 * b] = r1 * r2 - i1 * i2
        prod[2 * b + 1] = r1 * i2 + i1 * r2

    conv = irfft_scalar(prod, fft_len)
    return conv[:out_len]
