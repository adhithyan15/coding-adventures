"""Second-order section (biquad) filters and SOS cascade.

Conforms to DSP03 and VER15 security/weakness matrix invariants:
- Direct-Form-II Transposed biquad filter.
- Audio EQ Cookbook parametric filters (lowpass, highpass, bandpass, notch, etc.).
- Second-Order Sections (SOS) cascade filtering (sos_cascade, sosfilt).
- Pre-flight bounds checks (MAX_ORDER = 1024, MAX_SAMPLES = 16_777_216).
- CWE-20 finite float validation fail-closed.
- CWE-674 strictly iterative execution (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_filters.fir import (
    MAX_ORDER,
    MAX_SAMPLES,
    EmptySignalError,
    InvalidCoefficientError,
)


class Biquad:
    """Second-order IIR filter section using Direct-Form-II Transposed structure."""

    __slots__ = ("b0", "b1", "b2", "a1", "a2", "z1", "z2")

    def __init__(
        self,
        b0: float,
        b1: float,
        b2: float,
        a0: float,
        a1: float,
        a2: float,
    ) -> None:
        for val in (b0, b1, b2, a0, a1, a2):
            if not math.isfinite(val):
                raise InvalidCoefficientError("biquad coefficients must be finite")
        if a0 == 0.0:
            raise InvalidCoefficientError("a0 must be non-zero")

        inv_a0 = 1.0 / a0
        self.b0: float = b0 * inv_a0
        self.b1: float = b1 * inv_a0
        self.b2: float = b2 * inv_a0
        self.a1: float = a1 * inv_a0
        self.a2: float = a2 * inv_a0

        self.z1: float = 0.0
        self.z2: float = 0.0

    def reset(self) -> None:
        """Reset internal filter delay state."""
        self.z1 = 0.0
        self.z2 = 0.0

    def process_sample(self, x: float) -> float:
        """Process a single input sample through the biquad."""
        if not math.isfinite(x):
            raise InvalidCoefficientError("non-finite sample in biquad input")
        y = self.b0 * x + self.z1
        self.z1 = self.b1 * x - self.a1 * y + self.z2
        self.z2 = self.b2 * x - self.a2 * y
        return y

    def process(self, signal: Sequence[float]) -> list[float]:
        """Process a sequence of samples through the biquad."""
        n = len(signal)
        if n == 0:
            raise EmptySignalError("biquad input signal must be non-empty")
        if n > MAX_SAMPLES:
            raise InvalidCoefficientError(
                f"signal length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
            )

        out: list[float] = []
        for x in signal:
            out.append(self.process_sample(float(x)))
        return out

    # ─────────────────────────── Cookbook Designs ───────────────────────────

    @classmethod
    def lowpass(
        cls, cutoff_norm: float, q: float = 1.0 / math.sqrt(2.0)
    ) -> Biquad:
        """Create a low-pass biquad at cutoff_norm in (0.0, 0.5)."""
        if not math.isfinite(cutoff_norm) or not (0.0 < cutoff_norm < 0.5):
            raise InvalidCoefficientError("cutoff_norm must be in (0.0, 0.5)")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        omega = 2.0 * math.pi * cutoff_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = (1.0 - cos_w) * 0.5
        b1 = 1.0 - cos_w
        b2 = b0
        a0 = 1.0 + alpha
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha
        return cls(b0, b1, b2, a0, a1, a2)

    @classmethod
    def highpass(
        cls, cutoff_norm: float, q: float = 1.0 / math.sqrt(2.0)
    ) -> Biquad:
        """Create a high-pass biquad at cutoff_norm in (0.0, 0.5)."""
        if not math.isfinite(cutoff_norm) or not (0.0 < cutoff_norm < 0.5):
            raise InvalidCoefficientError("cutoff_norm must be in (0.0, 0.5)")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        omega = 2.0 * math.pi * cutoff_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = (1.0 + cos_w) * 0.5
        b1 = -(1.0 + cos_w)
        b2 = b0
        a0 = 1.0 + alpha
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha
        return cls(b0, b1, b2, a0, a1, a2)

    @classmethod
    def bandpass(cls, center_norm: float, q: float = 1.0) -> Biquad:
        """Create a band-pass biquad (constant skirt gain, peak gain = Q)."""
        if not math.isfinite(center_norm) or not (0.0 < center_norm < 0.5):
            raise InvalidCoefficientError("center_norm must be in (0.0, 0.5)")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        omega = 2.0 * math.pi * center_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = sin_w * 0.5
        b1 = 0.0
        b2 = -b0
        a0 = 1.0 + alpha
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha
        return cls(b0, b1, b2, a0, a1, a2)

    @classmethod
    def notch(cls, center_norm: float, q: float = 1.0) -> Biquad:
        """Create a notch (band-stop) biquad."""
        if not math.isfinite(center_norm) or not (0.0 < center_norm < 0.5):
            raise InvalidCoefficientError("center_norm must be in (0.0, 0.5)")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        omega = 2.0 * math.pi * center_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = 1.0
        b1 = -2.0 * cos_w
        b2 = 1.0
        a0 = 1.0 + alpha
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha
        return cls(b0, b1, b2, a0, a1, a2)

    @classmethod
    def allpass(cls, center_norm: float, q: float = 1.0) -> Biquad:
        """Create an all-pass biquad."""
        if not math.isfinite(center_norm) or not (0.0 < center_norm < 0.5):
            raise InvalidCoefficientError("center_norm must be in (0.0, 0.5)")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        omega = 2.0 * math.pi * center_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = 1.0 - alpha
        b1 = -2.0 * cos_w
        b2 = 1.0 + alpha
        a0 = 1.0 + alpha
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha
        return cls(b0, b1, b2, a0, a1, a2)

    @classmethod
    def peak(
        cls, center_norm: float, gain_db: float, q: float = 1.0
    ) -> Biquad:
        """Create a peaking EQ biquad."""
        if not math.isfinite(center_norm) or not (0.0 < center_norm < 0.5):
            raise InvalidCoefficientError("center_norm must be in (0.0, 0.5)")
        if not math.isfinite(gain_db):
            raise InvalidCoefficientError("gain_db must be finite")
        if not math.isfinite(q) or q <= 0.0:
            raise InvalidCoefficientError("Q must be finite and > 0")

        a_gain = 10.0 ** (gain_db / 40.0)
        omega = 2.0 * math.pi * center_norm
        sin_w = math.sin(omega)
        cos_w = math.cos(omega)
        alpha = sin_w / (2.0 * q)

        b0 = 1.0 + alpha * a_gain
        b1 = -2.0 * cos_w
        b2 = 1.0 - alpha * a_gain
        a0 = 1.0 + alpha / a_gain
        a1 = -2.0 * cos_w
        a2 = 1.0 - alpha / a_gain
        return cls(b0, b1, b2, a0, a1, a2)


# ─────────────────────────── SOS Cascade ───────────────────────────


def sos_cascade(
    signal: Sequence[float], sos: Sequence[Sequence[float]]
) -> list[float]:
    """Filter signal using a cascade of Second-Order Sections (SOS).

    sos is a sequence of length-6 coefficient arrays [b0, b1, b2, a0, a1, a2].
    Matches scipy.signal.sosfilt.
    """
    n = len(signal)
    if n == 0:
        raise EmptySignalError("signal must not be empty")
    if n > MAX_SAMPLES:
        raise InvalidCoefficientError(
            f"signal length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    num_sections = len(sos)
    if num_sections > MAX_ORDER:
        raise InvalidCoefficientError(
            f"section count {num_sections} exceeds MAX_ORDER ({MAX_ORDER})"
        )

    # Validate all samples in signal
    current = [float(x) for x in signal]
    for x in current:
        if not math.isfinite(x):
            raise InvalidCoefficientError("non-finite sample in signal")

    # Cascade through each biquad section
    for section_idx, sec in enumerate(sos):
        if len(sec) != 6:
            raise InvalidCoefficientError(
                f"section {section_idx} must have 6 coefficients; got {len(sec)}"
            )
        b0, b1, b2, a0, a1, a2 = (float(v) for v in sec)
        bq = Biquad(b0, b1, b2, a0, a1, a2)
        current = bq.process(current)

    return current


def sosfilt(
    sos: Sequence[Sequence[float]], signal: Sequence[float]
) -> list[float]:
    """Alias for sos_cascade matching scipy.signal.sosfilt argument order."""
    return sos_cascade(signal, sos)
