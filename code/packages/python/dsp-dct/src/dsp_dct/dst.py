"""Discrete Sine Transform (DST-I..IV).

Conforms to DSP02 and VER15 security/weakness matrix invariants:
- Complete support for DST types I, II, III, and IV.
- None (unnormalized) and Ortho (orthonormal) conventions matching scipy.fft.dst.
- Strict CWE-400 bounds and CWE-20 finite-float validation.
- Zero recursion (CWE-674).
- Zero regular expressions (CWE-1333).
"""

from __future__ import annotations

import enum
import math
from collections.abc import Sequence

from dsp_dct.dct import (
    DctNorm,
    InvalidInputError,
    _parse_norm,
    _validate_signal,
)


class DstType(enum.Enum):
    """Which DST variant to compute."""

    I = 1  # noqa: E741
    II = 2
    III = 3
    IV = 4


def _parse_dst_type(dst_type: DstType | int | str) -> DstType:
    """Normalize user-specified dst_type into DstType enum."""
    if isinstance(dst_type, DstType):
        return dst_type
    if isinstance(dst_type, int):
        for member in DstType:
            if member.value == dst_type:
                return member
    if isinstance(dst_type, str):
        upper = dst_type.strip().upper()
        for member in DstType:
            if member.name == upper or str(member.value) == upper:
                return member
    raise InvalidInputError(f"unknown DST type: {dst_type!r}")


def _dst_i(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DST-I."""
    denom = n + 1.0
    out: list[float] = []
    for k in range(n):
        acc = 0.0
        k_factor = k + 1.0
        for m in range(n):
            theta = math.pi * k_factor * (m + 1.0) / denom
            acc += float(signal[m]) * math.sin(theta)
        out.append(2.0 * acc)

    if norm == DctNorm.ORTHO:
        scale = math.sqrt(2.0 / denom) * 0.5
        for i in range(n):
            out[i] *= scale
    return out


def _dst_ii(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DST-II."""
    two_n = 2.0 * n
    out: list[float] = []
    for k in range(n):
        acc = 0.0
        k_factor = k + 1.0
        for m in range(n):
            theta = math.pi * k_factor * (2.0 * m + 1.0) / two_n
            acc += float(signal[m]) * math.sin(theta)
        out.append(2.0 * acc)

    if norm == DctNorm.ORTHO:
        sk = math.sqrt(1.0 / (2.0 * n))
        s_last = math.sqrt(1.0 / (4.0 * n))
        for i in range(n - 1):
            out[i] *= sk
        out[n - 1] *= s_last
    return out


def _dst_iii(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DST-III."""
    two_n = 2.0 * n
    out: list[float] = []
    x_last = float(signal[n - 1])

    for k in range(n):
        acc = ((-1.0) ** k) * x_last
        k_factor = 2.0 * k + 1.0
        for m in range(n - 1):
            theta = math.pi * k_factor * (m + 1.0) / two_n
            acc += 2.0 * float(signal[m]) * math.sin(theta)
        out.append(acc)

    if norm == DctNorm.ORTHO:
        sk = math.sqrt(1.0 / (2.0 * n))
        s0 = math.sqrt(1.0 / (4.0 * n))
        out[0] *= s0
        for i in range(1, n):
            out[i] *= sk
    return out


def _dst_iv(signal: Sequence[float], n: int, norm: DctNorm) -> list[float]:
    """Iterative DST-IV."""
    four_n = 4.0 * n
    out: list[float] = []
    for k in range(n):
        acc = 0.0
        k_factor = 2.0 * k + 1.0
        for m in range(n):
            theta = math.pi * k_factor * (2.0 * m + 1.0) / four_n
            acc += float(signal[m]) * math.sin(theta)
        out.append(2.0 * acc)

    if norm == DctNorm.ORTHO:
        scale = math.sqrt(2.0 / n) * 0.5
        for i in range(n):
            out[i] *= scale
    return out


def dst(
    signal: Sequence[float],
    dst_type: DstType | int | str = DstType.II,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Compute forward Discrete Sine Transform (DST-I..IV)."""
    n = _validate_signal(signal)
    t = _parse_dst_type(dst_type)
    nm = _parse_norm(norm)

    if t == DstType.I:
        return _dst_i(signal, n, nm)
    if t == DstType.II:
        return _dst_ii(signal, n, nm)
    if t == DstType.III:
        return _dst_iii(signal, n, nm)
    if t == DstType.IV:
        return _dst_iv(signal, n, nm)

    raise InvalidInputError(f"unsupported DST type: {t}")


def idst(
    coeffs: Sequence[float],
    dst_type: DstType | int | str = DstType.III,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Compute inverse Discrete Sine Transform.

    Passing DstType.III with the same norm inverts DstType.II.
    Passing DstType.II with the same norm inverts DstType.III.
    """
    n = _validate_signal(coeffs)
    t = _parse_dst_type(dst_type)
    nm = _parse_norm(norm)

    if t == DstType.III:
        # Invert DST-II
        if nm == DctNorm.ORTHO:
            sk = math.sqrt(2.0 * n)
            s_last = math.sqrt(4.0 * n)
            scaled = [float(coeffs[i]) * sk for i in range(n - 1)]
            scaled.append(float(coeffs[n - 1]) * s_last)
            raw = _dst_iii(scaled, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]
        else:
            raw = _dst_iii(coeffs, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]

    if t == DstType.II:
        # Invert DST-III
        if nm == DctNorm.ORTHO:
            s0 = math.sqrt(4.0 * n)
            sk = math.sqrt(2.0 * n)
            scaled = [float(coeffs[0]) * s0]
            for i in range(1, n):
                scaled.append(float(coeffs[i]) * sk)
            raw = _dst_ii(scaled, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]
        else:
            raw = _dst_ii(coeffs, n, DctNorm.NONE)
            inv_scale = 1.0 / (2.0 * n)
            return [v * inv_scale for v in raw]

    if t == DstType.I:
        # DST-I is self-inverse up to 2(N+1) scale
        if nm == DctNorm.ORTHO:
            return _dst_i(coeffs, n, DctNorm.ORTHO)
        else:
            raw = _dst_i(coeffs, n, DctNorm.NONE)
            scale = 1.0 / (2.0 * (n + 1))
            return [v * scale for v in raw]

    if t == DstType.IV:
        # DST-IV is self-inverse up to 2N scale
        if nm == DctNorm.ORTHO:
            return _dst_iv(coeffs, n, DctNorm.ORTHO)
        else:
            raw = _dst_iv(coeffs, n, DctNorm.NONE)
            scale = 1.0 / (2.0 * n)
            return [v * scale for v in raw]

    raise InvalidInputError(f"unsupported DST type: {t}")
