"""FFT-based 1-D convolution using dsp-fft."""

from __future__ import annotations

from collections.abc import Sequence

from dsp_fft.radix2 import MAX_FFT_LEN
from dsp_fft.rfft import irfft_scalar, rfft_scalar

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


def _next_pow2(n: int) -> int:
    """Return smallest power of 2 >= n."""
    if n <= 1:
        return 1
    return 1 << (n - 1).bit_length()


def fft_convolve(
    signal: Sequence[float],
    kernel: Sequence[float],
    mode: ConvMode | str = ConvMode.FULL,
) -> list[float]:
    """Compute 1-D convolution via FFT with O(N log N) complexity."""
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

    full_len = n + k - 1
    if full_len > MAX_SAMPLES:
        raise InvalidInputError(
            f"convolution output length {full_len} exceeds MAX_SAMPLES {MAX_SAMPLES}"
        )

    fft_len = _next_pow2(full_len)
    if fft_len > MAX_FFT_LEN:
        raise InvalidInputError(
            f"required FFT length {fft_len} exceeds MAX_FFT_LEN {MAX_FFT_LEN}"
        )

    sig_padded = [0.0] * fft_len
    ker_padded = [0.0] * fft_len
    for i in range(n):
        sig_padded[i] = float(signal[i])
    for j in range(k):
        ker_padded[j] = float(kernel[j])

    sig_spec = rfft_scalar(sig_padded)
    ker_spec = rfft_scalar(ker_padded)

    num_bins = fft_len // 2 + 1
    prod_spec = [0.0] * (num_bins * 2)
    for b in range(num_bins):
        re_s = sig_spec[2 * b]
        im_s = sig_spec[2 * b + 1]
        re_k = ker_spec[2 * b]
        im_k = ker_spec[2 * b + 1]
        prod_spec[2 * b] = re_s * re_k - im_s * im_k
        prod_spec[2 * b + 1] = re_s * im_k + im_s * re_k

    reconstructed = irfft_scalar(prod_spec, fft_len)
    full_result = reconstructed[:full_len]

    if c_mode == ConvMode.FULL:
        return full_result
    if c_mode == ConvMode.SAME:
        start = k // 2
        return full_result[start : start + n]
    # VALID
    start = k - 1
    end = start + (n - k + 1)
    return full_result[start:end]
