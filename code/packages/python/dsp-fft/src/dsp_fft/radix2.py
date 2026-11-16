"""Iterative Radix-2 Cooley-Tukey Fast Fourier Transform.

Conforms to DSP01 Phase 2 and VER15 security/weakness matrix invariants:
- CWE-400 resource exhaustion defense via MAX_FFT_LEN and MAX_SAMPLES.
- CWE-20 fail-closed finite float validation rejecting NaN/±Inf.
- CWE-674 strictly iterative butterflies and bit-reversal (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import enum
import math
from collections.abc import Sequence

# Maximum transform length for 1-D FFT operations (1,048,576 complex bins).
MAX_FFT_LEN: int = 1 << 20

# Maximum number of float samples allowed across any buffer (16,777,216 floats).
MAX_SAMPLES: int = 1 << 24


class FftError(ValueError):
    """Base exception for FFT operations."""


class InvalidInputError(FftError):
    """Raised on invalid input dimensions, bounds violations, or non-finite floats."""


class NotPowerOfTwoError(FftError):
    """Raised when an operation requiring power-of-two length receives non-pow2 N."""

    def __init__(self, n: int) -> None:
        super().__init__(
            f"FFT length {n} is not a power of two; use Bluestein for arbitrary N"
        )
        self.n = n


class Direction(enum.Enum):
    """FFT direction convention."""

    FORWARD = 1
    INVERSE = -1


def _validate_buffer(data: Sequence[float], name: str = "buffer") -> int:
    """Validate buffer finiteness, bounds, and even length pre-flight.

    Returns the number of complex elements n = len(data) // 2.
    """
    raw_len = len(data)
    if raw_len == 0:
        raise InvalidInputError(f"{name} length must be at least 1")
    if raw_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"{name} length {raw_len} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )
    if raw_len % 2 != 0:
        raise InvalidInputError(
            f"interleaved {name} must have even length; got {raw_len}"
        )

    n = raw_len // 2
    if n > MAX_FFT_LEN:
        raise InvalidInputError(
            f"{name} FFT length {n} exceeds MAX_FFT_LEN ({MAX_FFT_LEN})"
        )

    # Fail-closed finite float validation
    for x in data:
        val = float(x)
        if not math.isfinite(val):
            raise InvalidInputError(
                "input contains non-finite float (NaN or Inf)"
            )

    return n


def _require_pow2_length(n: int) -> None:
    """Validate that n is a power of two."""
    if n < 1:
        raise InvalidInputError("FFT length must be at least 1")
    if (n & (n - 1)) != 0:
        raise NotPowerOfTwoError(n)


def bit_reversal_permute(buf: list[float], n: int) -> None:
    """Reorder the interleaved buffer in-place by bit-reversed index.

    Strictly iterative algorithm (CWE-674).
    """
    bits = n.bit_length() - 1
    for i in range(n):
        # Calculate bit-reversed index iteratively
        r = 0
        x = i
        for _ in range(bits):
            r = (r << 1) | (x & 1)
            x >>= 1
        if r > i:
            buf[2 * i], buf[2 * r] = buf[2 * r], buf[2 * i]
            buf[2 * i + 1], buf[2 * r + 1] = buf[2 * r + 1], buf[2 * i + 1]


def butterflies(buf: list[float], n: int, dir: Direction) -> None:
    """Run iterative radix-2 butterfly stages on bit-reversed buffer in-place.

    Strictly iterative nested loops with no recursion (CWE-674).
    """
    sign = -1.0 if dir == Direction.FORWARD else 1.0
    full = 2
    while full <= n:
        half = full // 2
        for block_start in range(0, n, full):
            for j in range(half):
                theta = sign * 2.0 * math.pi * j / full
                w_re = math.cos(theta)
                w_im = math.sin(theta)

                x_idx = 2 * (block_start + j)
                y_idx = 2 * (block_start + j + half)

                x_re = buf[x_idx]
                x_im = buf[x_idx + 1]
                y_re = buf[y_idx]
                y_im = buf[y_idx + 1]

                # Complex multiplication: t = w * y
                t_re = w_re * y_re - w_im * y_im
                t_im = w_re * y_im + w_im * y_re

                buf[x_idx] = x_re + t_re
                buf[x_idx + 1] = x_im + t_im
                buf[y_idx] = x_re - t_re
                buf[y_idx + 1] = x_im - t_im

        full *= 2


def fft_scalar(signal: Sequence[float]) -> list[float]:
    """Forward FFT on interleaved [re, im, ...] data.

    Length of signal must be 2 * N where N is a power of two.
    Returns an interleaved buffer of the natural-order spectrum.
    """
    n = _validate_buffer(signal, "signal")
    _require_pow2_length(n)

    buf = [float(x) for x in signal]
    bit_reversal_permute(buf, n)
    butterflies(buf, n, Direction.FORWARD)
    return buf


def ifft_scalar(spectrum: Sequence[float]) -> list[float]:
    """Inverse FFT on interleaved [re, im, ...] data.

    Length of spectrum must be 2 * N where N is a power of two.
    Applies backward normalization (division by N).
    """
    n = _validate_buffer(spectrum, "spectrum")
    _require_pow2_length(n)

    buf = [float(x) for x in spectrum]
    bit_reversal_permute(buf, n)
    butterflies(buf, n, Direction.INVERSE)

    inv_n = 1.0 / n
    for i in range(len(buf)):
        buf[i] *= inv_n

    return buf
