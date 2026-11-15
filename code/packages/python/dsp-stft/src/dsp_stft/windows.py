"""Analysis and synthesis window functions."""

from __future__ import annotations

import math
from enum import StrEnum

from dsp_stft.errors import MAX_FFT_LEN, InvalidParamError


class WindowType(StrEnum):
    """Analysis window types supported by the STFT."""

    RECTANGULAR = "rectangular"
    HANN = "hann"
    HAMMING = "hamming"
    BLACKMAN = "blackman"


Rectangular = WindowType.RECTANGULAR
Hann = WindowType.HANN
Hamming = WindowType.HAMMING
Blackman = WindowType.BLACKMAN


def parse_window_type(window: WindowType | str) -> WindowType:
    """Normalize window type from enum or case-insensitive string."""
    if isinstance(window, WindowType):
        return window
    if isinstance(window, str):
        normalized = window.strip().lower()
        for member in WindowType:
            if member.value == normalized:
                return member
    raise InvalidParamError(f"unsupported window type: {window}")


def build_window(window: WindowType | str, n_fft: int) -> list[float]:
    """Build an analysis window of length n_fft per the WindowType.

    Matches scipy.signal.get_window with sym=True conventions.
    """
    if n_fft <= 0:
        raise InvalidParamError(f"n_fft must be > 0; got {n_fft}")
    if n_fft > MAX_FFT_LEN:
        raise InvalidParamError(
            f"n_fft {n_fft} exceeds MAX_FFT_LEN {MAX_FFT_LEN}"
        )

    w_type = parse_window_type(window)
    if n_fft == 1:
        return [1.0]

    m = float(n_fft - 1)
    two_pi = 2.0 * math.pi
    four_pi = 4.0 * math.pi

    out = [0.0] * n_fft
    for n in range(n_fft):
        nn = float(n)
        if w_type == WindowType.RECTANGULAR:
            out[n] = 1.0
        elif w_type == WindowType.HANN:
            out[n] = 0.5 * (1.0 - math.cos(two_pi * nn / m))
        elif w_type == WindowType.HAMMING:
            out[n] = 0.54 - 0.46 * math.cos(two_pi * nn / m)
        elif w_type == WindowType.BLACKMAN:
            out[n] = (
                0.42
                - 0.5 * math.cos(two_pi * nn / m)
                + 0.08 * math.cos(four_pi * nn / m)
            )
    return out


def rectangular_window(n: int) -> list[float]:
    """Return a rectangular window of length n."""
    return build_window(WindowType.RECTANGULAR, n)


def hann_window(n: int) -> list[float]:
    """Return a Hann window of length n."""
    return build_window(WindowType.HANN, n)


def hamming_window(n: int) -> list[float]:
    """Return a Hamming window of length n."""
    return build_window(WindowType.HAMMING, n)


def blackman_window(n: int) -> list[float]:
    """Return a Blackman window of length n."""
    return build_window(WindowType.BLACKMAN, n)
