"""Discrete Cosine Transform (DCT-I..IV) with O(N log N) fast paths via FFT.

Conforms to DSP02 and VER15 security/weakness matrix invariants:
- DCT-II via Makhoul reduction to FFT in O(N log N).
- DCT-III via inverse Makhoul reduction to IFFT in O(N log N).
- Complete support for DCT-I, DCT-II, DCT-III, and DCT-IV.
- Both None (unnormalized) and Ortho (orthonormal) conventions.
- CWE-400 bounds checks before allocating buffers.
- CWE-20 finite float validation fail-closed.
- CWE-674 strictly iterative execution (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import enum
import math
from collections.abc import Sequence

from dsp_fft import (
    Direction,
    bluestein_scalar,
    fft_scalar,
    ifft_scalar,
)

# Maximum 1-D DCT transform length (1,048,576 samples).
MAX_DCT_LEN: int = 1 << 20

# Maximum total samples across 1-D or 2-D buffers (16,777,216 samples).
MAX_SAMPLES: int = 1 << 24


class DctError(ValueError):
    """Base exception for DCT operations."""


class EmptyInputError(DctError):
    """Raised when transform input is empty."""


class InvalidInputError(DctError):
    """Raised on invalid dimensions, non-finite values, or bounds violations."""


class DctType(enum.Enum):
    """Which DCT variant to compute."""

    I = 1  # noqa: E741
    II = 2
    III = 3
    IV = 4


class DctNorm(enum.Enum):
    """Normalization convention."""

    NONE = 0
    ORTHO = 1


def _parse_norm(norm: DctNorm | str | int) -> DctNorm:
    """Normalize user-specified norm into DctNorm enum."""
    if isinstance(norm, DctNorm):
        return norm
    if isinstance(norm, int):
        if norm == 0:
            return DctNorm.NONE
        if norm == 1:
            return DctNorm.ORTHO
    if isinstance(norm, str):
        lowered = norm.strip().lower()
        if lowered in ("none", "raw", "unnorm"):
            return DctNorm.NONE
        if lowered in ("ortho", "unitary"):
            return DctNorm.ORTHO
    raise InvalidInputError(f"unknown DCT normalization: {norm!r}")


def _parse_type(dct_type: DctType | int | str) -> DctType:
    """Normalize user-specified dct_type into DctType enum."""
    if isinstance(dct_type, DctType):
        return dct_type
    if isinstance(dct_type, int):
        for member in DctType:
            if member.value == dct_type:
                return member
    if isinstance(dct_type, str):
        upper = dct_type.strip().upper()
        for member in DctType:
            if member.name == upper or str(member.value) == upper:
                return member
    raise InvalidInputError(f"unknown DCT type: {dct_type!r}")


def _validate_signal(signal: Sequence[float]) -> int:
    """Pre-flight check for non-empty, length bounds, and finite floats."""
    n = len(signal)
    if n == 0:
        raise EmptyInputError("input signal must not be empty")
    if n > MAX_DCT_LEN:
        raise InvalidInputError(
            f"input length {n} exceeds MAX_DCT_LEN ({MAX_DCT_LEN})"
        )
    if n > MAX_SAMPLES:
        raise InvalidInputError(
            f"input length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    for x in signal:
        if not math.isfinite(float(x)):
            raise InvalidInputError(
                "input contains non-finite float (NaN or Inf)"
            )
    return n


# ─────────────────────────── Fast O(N log N) Paths ───────────────────────────


def _dct_ii_via_fft(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Compute forward DCT-II in O(N log N) via Makhoul's reduction."""
    # Step 1: Makhoul shuffle
    half_even = (n + 1) // 2
    half_odd = n // 2
    y = [0.0] * n
    for m in range(half_even):
        y[m] = float(signal[2 * m])
    for m in range(half_odd):
        y[n - 1 - m] = float(signal[2 * m + 1])

    # Step 2: FFT of length N (interleaved complex with imag=0)
    interleaved: list[float] = []
    for v in y:
        interleaved.append(v)
        interleaved.append(0.0)

    is_pow2 = (n & (n - 1)) == 0 and n >= 1
    if is_pow2:
        spectrum = fft_scalar(interleaved)
    else:
        spectrum = bluestein_scalar(interleaved, Direction.FORWARD)

    # Step 3: Twiddle multiply, take real part, double
    two_n = 2.0 * n
    out: list[float] = []
    for k in range(n):
        theta = math.pi * k / two_n
        cos_t = math.cos(theta)
        sin_t = math.sin(theta)
        y_re = spectrum[2 * k]
        y_im = spectrum[2 * k + 1]
        out.append(2.0 * (y_re * cos_t + y_im * sin_t))

    # Step 4: Normalise
    if norm == DctNorm.ORTHO:
        s0 = math.sqrt(1.0 / (4.0 * n))
        sk = math.sqrt(1.0 / (2.0 * n))
        out[0] *= s0
        for i in range(1, n):
            out[i] *= sk

    return out


def _dct_iii_via_fft(coeffs: Sequence[float], n: int) -> list[float]:
    """Fast un-normalised DCT-III in O(N log N) via IFFT (inverse Makhoul)."""
    if n == 1:
        return [float(coeffs[0])]

    interleaved = [float(coeffs[0]), 0.0]
    two_n = 2.0 * n
    for k in range(1, n):
        theta = math.pi * k / two_n
        cos_t = math.cos(theta)
        sin_t = math.sin(theta)
        re_k = float(coeffs[k])
        im_k = -float(coeffs[n - k])
        # (re_k + i*im_k) * (cos_t + i*sin_t)
        y_re = re_k * cos_t - im_k * sin_t
        y_im = re_k * sin_t + im_k * cos_t
        interleaved.append(y_re)
        interleaved.append(y_im)

    is_pow2 = (n & (n - 1)) == 0 and n >= 1
    if is_pow2:
        y = ifft_scalar(interleaved)
    else:
        y = bluestein_scalar(interleaved, Direction.INVERSE)

    out = [0.0] * n
    half_even = (n + 1) // 2
    half_odd = n // 2
    for m in range(half_even):
        out[2 * m] = y[2 * m] * n
    for m in range(half_odd):
        out[2 * m + 1] = y[2 * (n - 1 - m)] * n

    return out


# ─────────────────────────── Textbook Iterative Forms ───────────────────────────


def _dct_i(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DCT-I (N >= 2)."""
    if n < 2:
        raise InvalidInputError("DCT-I requires input length >= 2")

    denom = n - 1
    if norm == DctNorm.ORTHO:
        # Pre-scale endpoints by 1/sqrt(2)
        inv_sqrt2 = 1.0 / math.sqrt(2.0)
        x_scaled = [float(x) for x in signal]
        x_scaled[0] *= inv_sqrt2
        x_scaled[n - 1] *= inv_sqrt2

        out: list[float] = []
        scale = math.sqrt(2.0 / denom)
        for k in range(n):
            w_k = inv_sqrt2 if (k == 0 or k == n - 1) else 1.0
            acc = 0.0
            for m in range(n):
                theta = math.pi * k * m / denom
                acc += x_scaled[m] * math.cos(theta)
            out.append(scale * w_k * acc)
        return out
    else:
        out: list[float] = []
        x0 = float(signal[0])
        x_last = float(signal[n - 1])
        for k in range(n):
            acc = x0 + ((-1.0) ** k) * x_last
            for m in range(1, n - 1):
                theta = math.pi * k * m / denom
                acc += 2.0 * float(signal[m]) * math.cos(theta)
            out.append(acc)
        return out


def _dct_iv(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DCT-IV."""
    four_n = 4.0 * n
    out: list[float] = []
    for k in range(n):
        acc = 0.0
        k_factor = 2.0 * k + 1.0
        for m in range(n):
            theta = math.pi * k_factor * (2.0 * m + 1.0) / four_n
            acc += float(signal[m]) * math.cos(theta)
        out.append(2.0 * acc)

    if norm == DctNorm.ORTHO:
        scale = math.sqrt(2.0 / n) * 0.5
        for i in range(n):
            out[i] *= scale

    return out


# ─────────────────────────── Public 1-D API ───────────────────────────


def dct(
    signal: Sequence[float],
    dct_type: DctType | int | str = DctType.II,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Forward Discrete Cosine Transform.

    Supports DCT types I, II, III, IV and norms None and Ortho.
    """
    n = _validate_signal(signal)
    t = _parse_type(dct_type)
    nm = _parse_norm(norm)

    if t == DctType.II:
        return _dct_ii_via_fft(signal, n, nm)
    if t == DctType.III:
        unnorm = _dct_iii_via_fft(signal, n)
        if nm == DctNorm.ORTHO:
            s0 = math.sqrt(1.0 / (4.0 * n))
            sk = math.sqrt(1.0 / (2.0 * n))
            unnorm[0] *= s0
            for i in range(1, n):
                unnorm[i] *= sk
        return unnorm
    if t == DctType.I:
        return _dct_i(signal, n, nm)
    if t == DctType.IV:
        return _dct_iv(signal, n, nm)

    raise InvalidInputError(f"unsupported DCT type: {t}")


def idct(
    coeffs: Sequence[float],
    dct_type: DctType | int | str = DctType.III,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Inverse Discrete Cosine Transform.

    Passing DctType.III with the same norm inverts DctType.II.
    Passing DctType.II with the same norm inverts DctType.III.
    """
    n = _validate_signal(coeffs)
    t = _parse_type(dct_type)
    nm = _parse_norm(norm)

    if t == DctType.III:
        # Invert DCT-II
        if nm == DctNorm.ORTHO:
            s0 = math.sqrt(4.0 * n)
            sk = math.sqrt(2.0 * n)
            scaled = [float(coeffs[0]) * s0]
            for i in range(1, n):
                scaled.append(float(coeffs[i]) * sk)
        else:
            scaled = [float(x) for x in coeffs]

        raw = _dct_iii_via_fft(scaled, n)
        inv_scale = 1.0 / (2.0 * n)
        return [v * inv_scale for v in raw]

    if t == DctType.II:
        # Invert DCT-III
        if nm == DctNorm.ORTHO:
            s0 = math.sqrt(4.0 * n)
            sk = math.sqrt(2.0 * n)
            scaled = [float(coeffs[0]) * s0]
            for i in range(1, n):
                scaled.append(float(coeffs[i]) * sk)
            raw = _dct_ii_via_fft(scaled, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]
        else:
            raw = _dct_ii_via_fft(coeffs, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]

    if t == DctType.I:
        # DCT-I is self-inverse up to 2(N-1) scale
        if nm == DctNorm.ORTHO:
            return _dct_i(coeffs, n, DctNorm.ORTHO)
        else:
            raw = _dct_i(coeffs, n, DctNorm.NONE)
            scale = 1.0 / (2.0 * (n - 1))
            return [v * scale for v in raw]

    if t == DctType.IV:
        # DCT-IV is self-inverse up to 2N scale
        if nm == DctNorm.ORTHO:
            return _dct_iv(coeffs, n, DctNorm.ORTHO)
        else:
            raw = _dct_iv(coeffs, n, DctNorm.NONE)
            scale = 1.0 / (2.0 * n)
            return [v * scale for v in raw]

    raise InvalidInputError(f"unsupported DCT type: {t}")
