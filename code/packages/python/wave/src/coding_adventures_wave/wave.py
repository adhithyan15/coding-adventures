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
"""

import math
import sys

from trig import PI, sin


class Wave:
    """A simple harmonic wave with amplitude, frequency, and phase."""

    def __init__(self, amplitude: float, frequency: float, phase: float = 0.0) -> None:
        amplitude = float(amplitude)
        frequency = float(frequency)
        phase = float(phase)

        if not all(math.isfinite(value) for value in (amplitude, frequency, phase)):
            raise ValueError("Wave parameters must be finite")

        if amplitude < 0:
            raise ValueError(
                f"Amplitude must be >= 0, got {amplitude}. "
                "Use a phase shift to invert the wave instead."
            )

        if frequency <= 0:
            raise ValueError(
                f"Frequency must be > 0, got {frequency}. "
                "A wave must oscillate to be a wave."
            )

        if frequency > sys.float_info.max / (2.0 * PI):
            raise ValueError("Angular frequency must be finite")

        self._amplitude = float(amplitude)
        self._frequency = float(frequency)
        self._phase = float(phase)

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
        return 1.0 / self._frequency

    def angular_frequency(self) -> float:
        return 2.0 * PI * self._frequency

    def evaluate(self, t: float) -> float:
        t = float(t)
        if not math.isfinite(t):
            raise ValueError("Time must be finite")
        if self._amplitude == 0.0:
            return 0.0

        two_pi = 2.0 * PI
        period = self.period()
        reduced_time = t if math.isinf(period) else math.fmod(t, period)
        reduced_phase = math.fmod(self._phase, two_pi)
        theta = two_pi * (self._frequency * reduced_time) + reduced_phase
        unit = max(-1.0, min(1.0, sin(theta)))
        return self._amplitude * unit

    def __repr__(self) -> str:
        return (
            f"Wave(amplitude={self._amplitude}, "
            f"frequency={self._frequency}, "
            f"phase={self._phase})"
        )
