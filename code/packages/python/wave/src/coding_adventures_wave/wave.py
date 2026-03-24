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

from trig import PI, sin


class Wave:
    """A simple harmonic wave with amplitude, frequency, and phase."""

    def __init__(self, amplitude: float, frequency: float, phase: float = 0.0) -> None:
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
        theta = 2.0 * PI * self._frequency * t + self._phase
        return self._amplitude * sin(theta)

    def __repr__(self) -> str:
        return (
            f"Wave(amplitude={self._amplitude}, "
            f"frequency={self._frequency}, "
            f"phase={self._phase})"
        )
