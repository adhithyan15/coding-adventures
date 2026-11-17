"""1-D convolution and correlation with boundary and shape modes."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_conv.boundary import (
    MAX_KERNEL_LEN,
    MAX_SAMPLES,
    BoundaryMode,
    ConvMode,
    EmptyKernelError,
    EmptySignalError,
    InvalidInputError,
    KernelTooLargeError,
    extend_index,
    parse_boundary_mode,
    parse_conv_mode,
    validate_floats,
)


def sample(signal: Sequence[float], idx: int, mode: BoundaryMode) -> float:
    """Read signal at a possibly out-of-bounds index per the chosen boundary mode."""
    mapped = extend_index(idx, len(signal), mode)
    return signal[mapped] if mapped is not None else 0.0


def conv1d(
    signal: Sequence[float],
    kernel: Sequence[float],
    mode: BoundaryMode | str = BoundaryMode.ZERO,
) -> list[float]:
    """Same-size 1-D convolution.

    Output length equals len(signal). The kernel is centred at
    kernel.len() // 2 (matching scipy.ndimage.convolve and Rust dsp-conv).
    """
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

    b_mode = parse_boundary_mode(mode)
    validate_floats(signal, "signal")
    validate_floats(kernel, "kernel")

    n = len(signal)
    k = len(kernel)
    centre = k // 2

    out: list[float] = [0.0] * n
    for i in range(n):
        acc = 0.0
        for j in range(k):
            src_idx = i + centre - j
            acc += kernel[j] * sample(signal, src_idx, b_mode)
        out[i] = acc
    return out


def convolve(
    signal: Sequence[float],
    kernel: Sequence[float],
    mode: ConvMode | str = ConvMode.FULL,
    boundary: BoundaryMode | str = BoundaryMode.ZERO,
) -> list[float]:
    """1-D discrete convolution supporting Full, Same, and Valid modes."""
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
    b_mode = parse_boundary_mode(boundary)
    validate_floats(signal, "signal")
    validate_floats(kernel, "kernel")

    n = len(signal)
    k = len(kernel)

    if c_mode == ConvMode.SAME:
        return conv1d(signal, kernel, mode=b_mode)

    if c_mode == ConvMode.VALID:
        if n < k:
            raise KernelTooLargeError(
                f"kernel length {k} exceeds signal length {n} in valid mode"
            )
        out_len = n - k + 1
        out = [0.0] * out_len
        for i in range(out_len):
            acc = 0.0
            for j in range(k):
                acc += kernel[j] * signal[i + k - 1 - j]
            out[i] = acc
        return out

    # Full convolution mode
    out_len = n + k - 1
    if out_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"full conv length {out_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )
    out = [0.0] * out_len
    for i in range(out_len):
        acc = 0.0
        for j in range(k):
            src_idx = i - j
            acc += kernel[j] * sample(signal, src_idx, b_mode)
        out[i] = acc
    return out


def correlate(
    signal: Sequence[float],
    kernel: Sequence[float],
    mode: ConvMode | str = ConvMode.FULL,
) -> list[float]:
    """1-D discrete cross-correlation.

    (signal * kernel)(n) = sum_k signal[k] * kernel[k - n].
    For real sequences, cross-correlation is convolution with the reversed kernel.
    """
    if len(signal) == 0:
        raise EmptySignalError("signal must be non-empty")
    if len(kernel) == 0:
        raise EmptyKernelError("kernel must be non-empty")
    reversed_kernel = list(reversed(kernel))
    return convolve(signal, reversed_kernel, mode=mode, boundary=BoundaryMode.ZERO)
