"""
Wave — Simple Harmonic Wave Model
=================================

This module models a sinusoidal wave:

    y(t) = A * sin(2 * pi * f * t + phi)

where amplitude A controls the wave's height, frequency f controls how quickly
it oscillates, and phase phi shifts where the wave starts in its cycle.

We intentionally depend on the local `trig` package rather than Python's
standard `math` module so this package stays connected to the repo's
first-principles layering.

VER15 Hardening:
- CWE-20 (Improper Input Validation): Rejects booleans, non-Real objects,
  huge integer/Rational float overflows, NaN, ±Inf, negative amplitude,
  non-positive frequency, and angular-frequency overflow.
- CWE-190 / CWE-400 / CWE-674: Bounded O(1) error formatting, period/phase
  reduction prior to trig evaluation, amplitude-clamped output, and strictly
  iterative evaluation.
"""

from __future__ import annotations

import math
import sys
from numbers import Real

from trig import PI, sin


def _safe_repr(value: object) -> str:
    """Bounded O(1) representation for error messages (CWE-190 / CWE-400)."""
    if isinstance(value, bool):
        return repr(value)
    if isinstance(value, int):
        if value < -1_000_000 or value > 1_000_000:
            return "<out-of-range int>"
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        if len(value) > 64:
            return f"<str len={len(value)}>"
        return repr(value)
    return f"<{type(value).__name__}>"


def _finite_float(name: str, value: object) -> float:
    """Validate that `value` is a finite Real (not bool) and convert to float."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(
            f"{name} must be finite real number, got {_safe_repr(value)}"
        )
    try:
        converted = float(value)
    except OverflowError as exc:
        raise ValueError(
            f"{name} overflows float range: {_safe_repr(value)}"
        ) from exc
    if not math.isfinite(converted):
        raise ValueError(f"{name} must be finite, got {_safe_repr(converted)}")
    return converted


class Wave:
    """A simple harmonic wave with amplitude, frequency, and phase."""

    def __init__(self, amplitude: float, frequency: float, phase: float = 0.0) -> None:
        amp = _finite_float("Wave parameter amplitude", amplitude)
        freq = _finite_float("Wave parameter frequency", frequency)
        ph = _finite_float("Wave parameter phase", phase)

        if amp < 0.0:
            raise ValueError(
                f"Amplitude must be >= 0, got {amp}. "
                "Use a phase shift to invert the wave instead."
            )

        if freq <= 0.0:
            raise ValueError(
                f"Frequency must be > 0, got {freq}. "
                "A wave must oscillate to be a wave."
            )

        if freq > sys.float_info.max / (2.0 * PI) or not math.isfinite(2.0 * PI * freq):
            raise ValueError("Angular frequency must be finite")

        self._amplitude = 0.0 if amp == 0.0 else amp
        self._frequency = freq
        self._phase = 0.0 if ph == 0.0 else ph

    def _validate_state(self) -> None:
        if (
            not math.isfinite(self._amplitude)
            or self._amplitude < 0.0
            or not math.isfinite(self._frequency)
            or self._frequency <= 0.0
            or self._frequency > sys.float_info.max / (2.0 * PI)
            or not math.isfinite(2.0 * PI * self._frequency)
            or not math.isfinite(self._phase)
        ):
            raise ValueError(
                "Wave parameters must be finite and within physical domain"
            )

    @property
    def amplitude(self) -> float:
        return self._amplitude

    @property
    def frequency(self) -> float:
        return self._frequency

    @property
    def phase(self) -> float:
        return self._phase

    def period(self) -> float:
        self._validate_state()
        return 1.0 / self._frequency

    def angular_frequency(self) -> float:
        self._validate_state()
        return 2.0 * PI * self._frequency

    def evaluate(self, t: float) -> float:
        self._validate_state()
        t_val = _finite_float("Time", t)
        if self._amplitude == 0.0:
            return 0.0

        two_pi = 2.0 * PI
        period = 1.0 / self._frequency
        reduced_time = t_val if math.isinf(period) else math.fmod(t_val, period)
        reduced_phase = math.fmod(self._phase, two_pi)
        theta = two_pi * (self._frequency * reduced_time) + reduced_phase
        unit = max(-1.0, min(1.0, sin(theta)))
        result = self._amplitude * unit
        if not math.isfinite(result):
            raise ValueError("Evaluated wave value must be finite")
        return 0.0 if result == 0.0 else result

    def __repr__(self) -> str:
        return (
            f"Wave(amplitude={self._amplitude}, "
            f"frequency={self._frequency}, "
            f"phase={self._phase})"
        )
