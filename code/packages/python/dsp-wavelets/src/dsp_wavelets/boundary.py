"""Wavelet boundary extension modes and indexing."""

from __future__ import annotations

from collections.abc import Sequence
from enum import StrEnum

from dsp_wavelets.errors import InvalidParamError


class WaveletBoundary(StrEnum):
    """Signal boundary extension mode outside [0, N)."""

    ZERO = "zero"
    REPLICATE = "replicate"
    REFLECT = "reflect"
    SYMMETRIC = "symmetric"
    PERIODIC = "periodic"


Zero = WaveletBoundary.ZERO
Replicate = WaveletBoundary.REPLICATE
Reflect = WaveletBoundary.REFLECT
Symmetric = WaveletBoundary.SYMMETRIC
Periodic = WaveletBoundary.PERIODIC


def parse_wavelet_boundary(
    boundary: WaveletBoundary | str,
) -> WaveletBoundary:
    """Normalize boundary mode from enum or case-insensitive string."""
    if isinstance(boundary, WaveletBoundary):
        return boundary
    if isinstance(boundary, str):
        normalized = boundary.strip().lower()
        for member in WaveletBoundary:
            if member.value == normalized:
                return member
    raise InvalidParamError(f"unsupported boundary mode: {boundary}")


def sample_with_boundary(
    signal: Sequence[float],
    idx: int,
    boundary: WaveletBoundary | str,
) -> float:
    """Sample signal[idx] with the requested boundary extension mode."""
    n = len(signal)
    if n == 0:
        return 0.0
    if 0 <= idx < n:
        return signal[idx]

    b_mode = parse_wavelet_boundary(boundary)
    if b_mode == WaveletBoundary.ZERO:
        return 0.0
    if b_mode == WaveletBoundary.REPLICATE:
        clamped = min(max(idx, 0), n - 1)
        return signal[clamped]
    if b_mode == WaveletBoundary.REFLECT:
        if n == 1:
            return signal[0]
        period = 2 * (n - 1)
        m = idx % period
        if m >= n:
            m = period - m
        return signal[m]
    if b_mode == WaveletBoundary.PERIODIC:
        m = idx % n
        return signal[m]
    if b_mode == WaveletBoundary.SYMMETRIC:
        # Reflect with edge repeat: period 2n
        period = 2 * n
        m = idx % period
        if m >= n:
            m = 2 * n - 1 - m
        return signal[m]
    raise InvalidParamError(f"unknown boundary: {boundary}")
