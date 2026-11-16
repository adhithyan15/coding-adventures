"""Bluestein's algorithm — arbitrary-length FFT via chirp Z-transform.

Conforms to DSP01 Phase 4a and VER15 security/weakness matrix invariants:
- Recasts length-N DFT as length-M linear convolution (M = next_pow2(2N - 1)).
- Strictly iterative execution (zero recursion, CWE-674).
- CWE-400 bounds on N and M before allocating work buffers.
- CWE-20 finite float validation fail-closed.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_fft.radix2 import (
    MAX_FFT_LEN,
    Direction,
    InvalidInputError,
    _validate_buffer,
    fft_scalar,
    ifft_scalar,
)


def build_chirp(n: int, sign: float) -> list[float]:
    """Build the chirp sequence chirp[k] = exp(sign * i * pi * k^2 / N).

    Reduces k^2 modulo 2N to maintain maximum numerical precision.
    Returns interleaved [re, im, ...] of length 2 * N.
    """
    two_n = 2 * n
    out: list[float] = []
    for k in range(n):
        k_sq = k * k
        residue = k_sq % two_n
        theta = sign * math.pi * residue / n
        out.append(math.cos(theta))
        out.append(math.sin(theta))
    return out


def bluestein_scalar(
    signal: Sequence[float], direction: Direction
) -> list[float]:
    """Compute forward or inverse FFT of arbitrary length N >= 1 via Bluestein.

    signal is interleaved [re_0, im_0, ...] of length 2 * N.
    Returns an interleaved buffer of the resulting spectrum or signal.
    """
    n = _validate_buffer(signal, "signal")

    # Trivial N = 1 case: identity transform
    if n == 1:
        return [float(signal[0]), float(signal[1])]

    # Step 0: pick convolution length M = next_pow2(2N - 1)
    conv_len = 2 * n - 1
    m = 1 << (conv_len - 1).bit_length()
    if m > MAX_FFT_LEN:
        raise InvalidInputError(
            f"Bluestein convolution length {m} exceeds MAX_FFT_LEN ({MAX_FFT_LEN})"
        )

    # Step 1: build chirp
    sign = -1.0 if direction == Direction.FORWARD else 1.0
    chirp = build_chirp(n, sign)

    # Step 2: a[k] = signal[k] * chirp[k], padded to length M with zeros
    a = [0.0] * (2 * m)
    for k in range(n):
        xr = float(signal[2 * k])
        xi = float(signal[2 * k + 1])
        cr = chirp[2 * k]
        ci = chirp[2 * k + 1]
        # (xr + i*xi) * (cr + i*ci)
        a[2 * k] = xr * cr - xi * ci
        a[2 * k + 1] = xr * ci + xi * cr

    # Step 3: build bilateral wrapped chirp b' of length M
    # b'[k] = conj(chirp[k]) for k = 0..n-1
    # b'[m - k] = conj(chirp[k]) for k = 1..n-1
    b = [0.0] * (2 * m)
    for k in range(n):
        b[2 * k] = chirp[2 * k]
        b[2 * k + 1] = -chirp[2 * k + 1]

    for k in range(1, n):
        idx = m - k
        b[2 * idx] = chirp[2 * k]
        b[2 * idx + 1] = -chirp[2 * k + 1]

    # Step 4: convolve via radix-2 FFT
    a_spec = fft_scalar(a)
    b_spec = fft_scalar(b)

    c_spec = [0.0] * (2 * m)
    for k in range(m):
        ar = a_spec[2 * k]
        ai = a_spec[2 * k + 1]
        br = b_spec[2 * k]
        bi = b_spec[2 * k + 1]
        c_spec[2 * k] = ar * br - ai * bi
        c_spec[2 * k + 1] = ar * bi + ai * br

    conv = ifft_scalar(c_spec)

    # Step 5: X[k] = chirp[k] * conv[k], with 1/N scaling if inverse
    inv_n = 1.0 / n if direction == Direction.INVERSE else 1.0
    out = [0.0] * (2 * n)
    for k in range(n):
        cr = conv[2 * k]
        ci = conv[2 * k + 1]
        wr = chirp[2 * k]
        wi = chirp[2 * k + 1]
        re = wr * cr - wi * ci
        im = wr * ci + wi * cr
        out[2 * k] = re * inv_n
        out[2 * k + 1] = im * inv_n

    return out
