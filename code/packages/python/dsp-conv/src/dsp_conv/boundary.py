"""Boundary extension modes, convolution modes, defensive bounds, and errors."""

from __future__ import annotations

import math
from collections.abc import Sequence
from enum import StrEnum

# CWE-400 defensive bounds matching Rust and warrant specs
MAX_SAMPLES: int = 1 << 24  # 16,777,216 samples (~64 MB of float32)
MAX_KERNEL_LEN: int = 1 << 20  # 1,048,576 taps


class BoundaryMode(StrEnum):
    """Boundary extension mode outside [0, N) signal support."""

    ZERO = "zero"
    REPLICATE = "replicate"
    REFLECT = "reflect"
    WRAP = "wrap"


Zero = BoundaryMode.ZERO
Replicate = BoundaryMode.REPLICATE
Reflect = BoundaryMode.REFLECT
Wrap = BoundaryMode.WRAP


class ConvMode(StrEnum):
    """Convolution output shape mode."""

    FULL = "full"
    SAME = "same"
    VALID = "valid"


Full = ConvMode.FULL
Same = ConvMode.SAME
Valid = ConvMode.VALID


class ConvError(Exception):
    """Base exception for all convolution errors."""


class EmptySignalError(ConvError):
    """Raised when the input signal or image is empty."""


class EmptyKernelError(ConvError):
    """Raised when the convolution kernel is empty."""


class ImageSizeMismatchError(ConvError):
    """Raised when image or kernel dimensions do not match the buffer length."""


class KernelTooLargeError(ConvError):
    """Raised when the kernel is larger than the signal or image."""


class InvalidInputError(ConvError):
    """Raised on non-finite floats, defensive cap violations, or invalid arguments."""


def parse_boundary_mode(mode: BoundaryMode | str) -> BoundaryMode:
    """Normalize boundary mode from enum or case-insensitive string."""
    if isinstance(mode, BoundaryMode):
        return mode
    if isinstance(mode, str):
        normalized = mode.strip().lower()
        for member in BoundaryMode:
            if member.value == normalized:
                return member
    raise InvalidInputError(f"unsupported boundary mode: {mode}")


def parse_conv_mode(mode: ConvMode | str) -> ConvMode:
    """Normalize convolution mode from enum or case-insensitive string."""
    if isinstance(mode, ConvMode):
        return mode
    if isinstance(mode, str):
        normalized = mode.strip().lower()
        for member in ConvMode:
            if member.value == normalized:
                return member
    raise InvalidInputError(f"unsupported conv mode: {mode}")


def extend_index(idx: int, n: int, mode: BoundaryMode) -> int | None:
    """Map a possibly out-of-bounds index into [0, n) per mode, or None for Zero."""
    if 0 <= idx < n:
        return idx
    if mode == BoundaryMode.ZERO:
        return None
    if mode == BoundaryMode.REPLICATE:
        return min(max(idx, 0), n - 1)
    if mode == BoundaryMode.REFLECT:
        if n == 1:
            return 0
        period = 2 * (n - 1)
        m = idx % period
        if m >= n:
            m = period - m
        return m
    if mode == BoundaryMode.WRAP:
        return idx % n
    raise InvalidInputError(f"unknown boundary mode: {mode}")


def validate_floats(seq: Sequence[float], name: str) -> None:
    """Validate that all items in seq are finite floats (CWE-20)."""
    for idx, val in enumerate(seq):
        if not math.isfinite(val):
            raise InvalidInputError(
                f"non-finite float in {name} at index {idx}: {val}"
            )
