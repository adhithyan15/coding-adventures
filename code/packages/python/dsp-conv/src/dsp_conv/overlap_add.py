"""Overlap-add block convolution algorithm."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_conv.boundary import (
    MAX_KERNEL_LEN,
    MAX_SAMPLES,
    ConvMode,
    EmptyKernelError,
    EmptySignalError,
    InvalidInputError,
    KernelTooLargeError,
    parse_conv_mode,
    validate_floats,
)
from dsp_conv.fft_conv import fft_convolve


def overlap_add(
    signal: Sequence[float],
    kernel: Sequence[float],
    block_size: int | None = None,
    mode: ConvMode | str = ConvMode.FULL,
) -> list[float]:
    """Compute convolution using the overlap-add block method."""
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if len(kernel) == 0:
        raise EmptyKernelError("kernel must be non-empty")
    if len(signal) > MAX_SAMPLES:
        raise InvalidInputError(
            f"signal length {len(signal)} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    if len(kernel) > MAX_KERNEL_LEN:
        raise InvalidInputError(
            f"kernel length {len(kernel)} exceeds MAX_KERNEL_LEN {MAX_KERNEL_LEN}"
        )

    c_mode = parse_conv_mode(mode)
    validate_floats(signal, "signal")
    validate_floats(kernel, "kernel")

    n = len(signal)
    k = len(kernel)

    if c_mode == ConvMode.VALID and n < k:
        raise KernelTooLargeError(
            f"kernel length {k} exceeds signal length {n} in valid mode"
        )

    if block_size is None:
        block_len = max(64, 1 << (k.bit_length()))
    else:
        if block_size <= 0:
            raise InvalidInputError(f"block_size must be > 0; got {block_size}")
        if block_size > MAX_SAMPLES:
            raise InvalidInputError(
                f"block_size {block_size} exceeds MAX_SAMPLES {MAX_SAMPLES}"
            )
        block_len = block_size

    full_len = n + k - 1
    if full_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"convolution output length {full_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    out = [0.0] * full_len

    # Process blocks iteratively (zero recursion, CWE-674)
    start = 0
    while start < n:
        end = min(start + block_len, n)
        block = signal[start:end]
        conv_block = fft_convolve(block, kernel, mode=ConvMode.FULL)
        for idx, val in enumerate(conv_block):
            out_pos = start + idx
            if out_pos < full_len:
                out[out_pos] += val
        start += block_len

    if c_mode == ConvMode.FULL:
        return out
    if c_mode == ConvMode.SAME:
        offset = k // 2
        return out[offset : offset + n]
    # VALID
    offset = k - 1
    return out[offset : offset + (n - k + 1)]
