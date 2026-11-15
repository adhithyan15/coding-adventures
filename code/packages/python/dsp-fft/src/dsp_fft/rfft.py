"""Real-input FFT (rfft), inverse (irfft), and high-level ComplexTensor dispatch.

Conforms to DSP01 Phase 4b and VER15 security/weakness matrix invariants:
- Exploits conjugate symmetry X[N - k] = conj(X[k]) for real signals.
- Returns half-spectrum of floor(N / 2) + 1 complex bins.
- Strict CWE-400 bounds and CWE-20 finite-float validation.
- Zero recursion (CWE-674).
- Zero regular expressions (CWE-1333).
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_complex import ComplexTensor

from dsp_fft.bluestein import bluestein_scalar
from dsp_fft.radix2 import (
    MAX_FFT_LEN,
    MAX_SAMPLES,
    Direction,
    InvalidInputError,
    fft_scalar,
    ifft_scalar,
)


def rfft_scalar(signal: Sequence[float]) -> list[float]:
    """Compute forward FFT of a real-valued signal, returning half-spectrum.

    Returns interleaved [re, im, ...] of length 2 * (floor(N / 2) + 1).
    """
    n = len(signal)
    if n == 0:
        raise InvalidInputError("rfft input must be at least 1 sample")
    if n > MAX_FFT_LEN:
        raise InvalidInputError(
            f"rfft input length {n} exceeds MAX_FFT_LEN ({MAX_FFT_LEN})"
        )
    if n > MAX_SAMPLES:
        raise InvalidInputError(
            f"rfft input length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    # Wrap real signal into interleaved complex [x_0, 0, x_1, 0, ...]
    interleaved: list[float] = []
    for x in signal:
        val = float(x)
        if not math.isfinite(val):
            raise InvalidInputError(
                "input contains non-finite float (NaN or Inf)"
            )
        interleaved.append(val)
        interleaved.append(0.0)

    # Full FFT: radix-2 if pow2, otherwise Bluestein
    is_pow2 = (n & (n - 1)) == 0 and n >= 1
    if is_pow2:
        full = fft_scalar(interleaved)
    else:
        full = bluestein_scalar(interleaved, Direction.FORWARD)

    half_bins = n // 2 + 1
    return full[: 2 * half_bins]


def irfft_scalar(
    half_spectrum: Sequence[float], output_length: int
) -> list[float]:
    """Inverse FFT of a half-spectrum back to a real-valued signal of length N."""
    n = output_length
    if n < 1:
        raise InvalidInputError("irfft output length must be at least 1")
    if n > MAX_FFT_LEN:
        raise InvalidInputError(
            f"output length {n} exceeds MAX_FFT_LEN ({MAX_FFT_LEN})"
        )
    if len(half_spectrum) > MAX_SAMPLES:
        msg = (
            f"half_spectrum length {len(half_spectrum)} exceeds "
            f"MAX_SAMPLES ({MAX_SAMPLES})"
        )
        raise InvalidInputError(msg)
    if len(half_spectrum) % 2 != 0:
        raise InvalidInputError(
            f"irfft half-spectrum must have even length; got {len(half_spectrum)}"
        )

    half_bins = len(half_spectrum) // 2
    expected_half_bins = n // 2 + 1
    if half_bins != expected_half_bins:
        raise InvalidInputError(
            f"irfft: half-spectrum has {half_bins} bins, expected {expected_half_bins} "
            f"for output_length = {n}"
        )

    # Validate finite floats
    for x in half_spectrum:
        if not math.isfinite(float(x)):
            raise InvalidInputError(
                "input contains non-finite float (NaN or Inf)"
            )

    # Reconstruct full length-N spectrum using X[N - k] = conj(X[k])
    full = [0.0] * (2 * n)
    for i in range(2 * half_bins):
        full[i] = float(half_spectrum[i])

    # Reflect bins 1..(n + 1)//2 onto n - k with conjugation
    reflect_end = (n + 1) // 2
    for k in range(1, reflect_end):
        src_re = full[2 * k]
        src_im = full[2 * k + 1]
        dst = n - k
        full[2 * dst] = src_re
        full[2 * dst + 1] = -src_im

    is_pow2 = (n & (n - 1)) == 0 and n >= 1
    if is_pow2:
        inverted = ifft_scalar(full)
    else:
        inverted = bluestein_scalar(full, Direction.INVERSE)

    # Discard imaginary lane, keeping only real components
    return [inverted[2 * k] for k in range(n)]


def fft(
    signal: Sequence[float], complex: bool = False
) -> ComplexTensor:
    """Compute forward 1-D FFT returning a ComplexTensor.

    If complex=True, signal is interleaved [re, im] of length 2N.
    If complex=False, signal is real of length N.
    """
    if complex:
        raw_len = len(signal)
        if raw_len % 2 != 0:
            raise InvalidInputError(
                f"interleaved complex buffer must have even length; got {raw_len}"
            )
        n = raw_len // 2
        if n < 1:
            raise InvalidInputError("FFT length must be at least 1")
        if n > MAX_FFT_LEN or raw_len > MAX_SAMPLES:
            raise InvalidInputError(
                f"signal length {raw_len} exceeds maximum allowed"
            )

        for x in signal:
            if not math.isfinite(float(x)):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )

        if (n & (n - 1)) == 0:
            out = fft_scalar(signal)
        else:
            out = bluestein_scalar(signal, Direction.FORWARD)
    else:
        n = len(signal)
        if n < 1:
            raise InvalidInputError("FFT length must be at least 1")
        if n > MAX_FFT_LEN or n > MAX_SAMPLES:
            raise InvalidInputError(
                f"signal length {n} exceeds maximum allowed"
            )

        interleaved: list[float] = []
        for x in signal:
            val = float(x)
            if not math.isfinite(val):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )
            interleaved.append(val)
            interleaved.append(0.0)

        if (n & (n - 1)) == 0:
            out = fft_scalar(interleaved)
        else:
            out = bluestein_scalar(interleaved, Direction.FORWARD)

    return ComplexTensor.from_interleaved(out)


def ifft(spectrum: ComplexTensor) -> ComplexTensor:
    """Compute inverse 1-D FFT from a ComplexTensor."""
    n = len(spectrum)
    if n < 1:
        raise InvalidInputError("FFT length must be at least 1")
    if n > MAX_FFT_LEN:
        raise InvalidInputError(
            f"spectrum length {n} exceeds MAX_FFT_LEN ({MAX_FFT_LEN})"
        )

    interleaved = spectrum.as_interleaved()
    if (n & (n - 1)) == 0:
        out = ifft_scalar(interleaved)
    else:
        out = bluestein_scalar(interleaved, Direction.INVERSE)

    return ComplexTensor.from_interleaved(out)


def rfft(signal: Sequence[float]) -> ComplexTensor:
    """Compute real-input FFT returning a ComplexTensor half-spectrum."""
    interleaved = rfft_scalar(signal)
    return ComplexTensor.from_interleaved(interleaved)


def irfft(
    half_spectrum: ComplexTensor, output_length: int
) -> list[float]:
    """Compute inverse real-input FFT returning a real float list."""
    return irfft_scalar(half_spectrum.as_interleaved(), output_length)
