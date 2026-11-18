"""Filter design helpers: windowed-sinc FIR, Butterworth IIR, and Chebyshev-I IIR.

Conforms to DSP03 Phase 5 and VER15 security/weakness matrix invariants:
- Linear-phase windowed-sinc lowpass, highpass, and bandpass FIR filters.
- Bilinear transform Butterworth lowpass (orders 1 and 2).
- Bilinear transform Chebyshev Type I lowpass (orders 1 and 2).
- Pre-flight validation on cutoff frequencies, tap counts, and ripple.
- Zero recursion (CWE-674).
- Zero regular expressions (CWE-1333).
"""

from __future__ import annotations

import enum
import math

from dsp_filters.fir import MAX_TAPS, InvalidCoefficientError


class WindowType(enum.Enum):
    """Choice of window function for windowed-sinc FIR designs."""

    RECTANGULAR = "rectangular"
    HAMMING = "hamming"
    HANN = "hann"
    BLACKMAN = "blackman"


def _parse_window(window: WindowType | str) -> WindowType:
    if isinstance(window, WindowType):
        return window
    if isinstance(window, str):
        lowered = window.strip().lower()
        for member in WindowType:
            if member.value == lowered:
                return member
    raise InvalidCoefficientError(f"unknown window type: {window!r}")


# ─────────────────────────── FIR Designs ───────────────────────────


def design_low_pass(
    cutoff_norm: float,
    num_taps: int,
    window: WindowType | str = WindowType.HAMMING,
) -> list[float]:
    """Design a linear-phase low-pass FIR filter via windowed-sinc.

    cutoff_norm: normalised cutoff in [0.0, 0.5] (0.5 is Nyquist).
    num_taps: filter length — must be odd and >= 1.
    window: WindowType or window name string.

    Returns kernel normalized so DC gain is 1.0.
    """
    if num_taps < 1 or num_taps % 2 == 0:
        raise InvalidCoefficientError(
            f"num_taps must be odd and >= 1; got {num_taps}"
        )
    if num_taps > MAX_TAPS:
        raise InvalidCoefficientError(
            f"num_taps {num_taps} exceeds MAX_TAPS ({MAX_TAPS})"
        )
    if not math.isfinite(cutoff_norm) or not (0.0 <= cutoff_norm <= 0.5):
        raise InvalidCoefficientError(
            f"cutoff_norm must be finite in [0.0, 0.5]; got {cutoff_norm}"
        )

    win = _parse_window(window)
    m = float(num_taps - 1)
    centre = m / 2.0
    fc = float(cutoff_norm)

    h: list[float] = []
    for k in range(num_taps):
        x = float(k) - centre
        val = 2.0 * fc if x == 0.0 else math.sin(2.0 * math.pi * fc * x) / (math.pi * x)
        h.append(val)

    # Apply window
    for k in range(num_taps):
        if m == 0.0:
            w = 1.0
        else:
            n_f = float(k)
            if win == WindowType.RECTANGULAR:
                w = 1.0
            elif win == WindowType.HAMMING:
                w = 0.54 - 0.46 * math.cos(2.0 * math.pi * n_f / m)
            elif win == WindowType.HANN:
                w = 0.5 * (1.0 - math.cos(2.0 * math.pi * n_f / m))
            elif win == WindowType.BLACKMAN:
                w = (
                    0.42
                    - 0.5 * math.cos(2.0 * math.pi * n_f / m)
                    + 0.08 * math.cos(4.0 * math.pi * n_f / m)
                )
            else:
                w = 1.0
        h[k] *= w

    # Normalise DC gain = 1.0
    s = sum(h)
    if abs(s) > 0.0:
        inv_s = 1.0 / s
        # Retain legacy rounding unless the finite sum's reciprocal overflowed.
        if math.isfinite(inv_s) or not math.isfinite(s):
            for k in range(num_taps):
                h[k] *= inv_s
        else:
            for k in range(num_taps):
                h[k] /= s

    return h


def design_high_pass(
    cutoff_norm: float,
    num_taps: int,
    window: WindowType | str = WindowType.HAMMING,
) -> list[float]:
    """Design a linear-phase high-pass FIR filter via spectral inversion."""
    hp = [-v for v in design_low_pass(cutoff_norm, num_taps, window)]
    centre = (num_taps - 1) // 2
    hp[centre] += 1.0
    return hp


def design_band_pass(
    low_cutoff_norm: float,
    high_cutoff_norm: float,
    num_taps: int,
    window: WindowType | str = WindowType.HAMMING,
) -> list[float]:
    """Design a linear-phase band-pass FIR filter via windowed-sinc."""
    if num_taps < 1 or num_taps % 2 == 0:
        raise InvalidCoefficientError(
            f"num_taps must be odd and >= 1; got {num_taps}"
        )
    if num_taps > MAX_TAPS:
        raise InvalidCoefficientError(
            f"num_taps {num_taps} exceeds MAX_TAPS ({MAX_TAPS})"
        )
    if not math.isfinite(low_cutoff_norm) or not math.isfinite(high_cutoff_norm):
        raise InvalidCoefficientError("cutoff frequencies must be finite")
    if not (0.0 <= low_cutoff_norm < high_cutoff_norm <= 0.5):
        raise InvalidCoefficientError(
            f"cutoffs must satisfy 0.0 <= low < high <= 0.5; "
            f"got low={low_cutoff_norm}, high={high_cutoff_norm}"
        )

    win = _parse_window(window)
    m = float(num_taps - 1)
    centre = m / 2.0
    f1 = float(low_cutoff_norm)
    f2 = float(high_cutoff_norm)

    h: list[float] = []
    for k in range(num_taps):
        x = float(k) - centre
        if x == 0.0:
            val = 2.0 * (f2 - f1)
        else:
            val = (
                math.sin(2.0 * math.pi * f2 * x)
                - math.sin(2.0 * math.pi * f1 * x)
            ) / (math.pi * x)
        h.append(val)

    # Apply window
    for k in range(num_taps):
        if m == 0.0:
            w = 1.0
        else:
            n_f = float(k)
            if win == WindowType.RECTANGULAR:
                w = 1.0
            elif win == WindowType.HAMMING:
                w = 0.54 - 0.46 * math.cos(2.0 * math.pi * n_f / m)
            elif win == WindowType.HANN:
                w = 0.5 * (1.0 - math.cos(2.0 * math.pi * n_f / m))
            elif win == WindowType.BLACKMAN:
                w = (
                    0.42
                    - 0.5 * math.cos(2.0 * math.pi * n_f / m)
                    + 0.08 * math.cos(4.0 * math.pi * n_f / m)
                )
            else:
                w = 1.0
        h[k] *= w

    # Normalise for unit peak passband gain at centre frequency f0
    f0 = (f1 + f2) * 0.5
    gain = 0.0
    for k, h_k in enumerate(h):
        x = float(k) - centre
        gain += h_k * math.cos(2.0 * math.pi * f0 * x)

    gain_abs = abs(gain)
    if gain_abs > 0.0:
        inv_g = 1.0 / gain_abs
        # Retain legacy rounding unless the finite gain's reciprocal overflowed.
        if math.isfinite(inv_g) or not math.isfinite(gain_abs):
            for k in range(num_taps):
                h[k] *= inv_g
        else:
            for k in range(num_taps):
                h[k] /= gain_abs

    return h


# ─────────────────────────── IIR Designs ───────────────────────────


def butterworth_lowpass(
    order: int, cutoff_norm: float
) -> tuple[list[float], list[float]]:
    """Design a Butterworth low-pass IIR filter (orders 1 and 2).

    Returns (b, a) coefficient vectors.
    """
    if order not in (1, 2):
        raise InvalidCoefficientError(
            f"Butterworth supports orders 1 and 2; got {order}"
        )
    if not math.isfinite(cutoff_norm) or not (0.0 < cutoff_norm < 0.5):
        raise InvalidCoefficientError(
            f"cutoff_norm must be in (0.0, 0.5); got {cutoff_norm}"
        )

    k = math.tan(math.pi * cutoff_norm)

    if order == 1:
        alpha = k / (1.0 + k)
        return ([alpha, alpha], [1.0, 2.0 * alpha - 1.0])

    # order == 2
    omega0 = 2.0 * math.pi * cutoff_norm
    sin_w0 = math.sin(omega0)
    cos_w0 = math.cos(omega0)
    q = 1.0 / math.sqrt(2.0)
    alpha = sin_w0 / (2.0 * q)
    b0 = (1.0 - cos_w0) / 2.0
    b1 = 1.0 - cos_w0
    b2 = b0
    a0 = 1.0 + alpha
    a1 = -2.0 * cos_w0
    a2 = 1.0 - alpha

    return (
        [b0 / a0, b1 / a0, b2 / a0],
        [1.0, a1 / a0, a2 / a0],
    )


def chebyshev1_lowpass(
    order: int, ripple_db: float, cutoff_norm: float
) -> tuple[list[float], list[float]]:
    """Design a Chebyshev Type I low-pass IIR filter (orders 1 and 2).

    ripple_db: passband ripple in decibels (> 0.0).
    cutoff_norm: normalised cutoff in (0.0, 0.5).
    Returns (b, a) coefficient vectors.
    """
    if order not in (1, 2):
        raise InvalidCoefficientError(
            f"Chebyshev-I supports orders 1 and 2; got {order}"
        )
    if not math.isfinite(ripple_db) or ripple_db <= 0.0:
        raise InvalidCoefficientError(
            f"ripple_db must be finite and > 0; got {ripple_db}"
        )
    if not math.isfinite(cutoff_norm) or not (0.0 < cutoff_norm < 0.5):
        raise InvalidCoefficientError(
            f"cutoff_norm must be in (0.0, 0.5); got {cutoff_norm}"
        )

    k = math.tan(math.pi * cutoff_norm)
    eps = math.sqrt(10.0 ** (ripple_db * 0.1) - 1.0)
    mu = (1.0 / order) * math.log((1.0 / eps) + math.sqrt((1.0 / (eps * eps)) + 1.0))

    if order == 1:
        sinh_mu = math.sinh(mu)
        alpha = k * sinh_mu / (1.0 + k * sinh_mu)
        return ([alpha, alpha], [1.0, 2.0 * alpha - 1.0])

    # order == 2
    u = math.sinh(mu) / math.sqrt(2.0)
    v = math.cosh(mu) / math.sqrt(2.0)
    sum_sq = u * u + v * v
    na = sum_sq / math.sqrt(1.0 + eps * eps)

    a0 = 1.0 + 2.0 * u * k + sum_sq * k * k
    a1 = 2.0 * (sum_sq * k * k - 1.0)
    a2 = 1.0 - 2.0 * u * k + sum_sq * k * k

    b0 = na * k * k
    b1 = 2.0 * b0
    b2 = b0

    return (
        [b0 / a0, b1 / a0, b2 / a0],
        [1.0, a1 / a0, a2 / a0],
    )
