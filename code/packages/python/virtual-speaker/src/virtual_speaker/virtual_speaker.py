"""Turn virtual analog input into a speaker-pressure proxy.

This package is intentionally not a physics simulation yet. It exists so the
note-to-sound chain has a named, reusable boundary where voltage-like values
become sound-pressure-like values.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import isfinite
from numbers import Real
from typing import Protocol

DEFAULT_SPEAKER_GAIN = 1.0


class AnalogSignal(Protocol):
    """A virtual analog signal that can be queried at any time in seconds."""

    def value_at(self, time_seconds: Real) -> float:
        """Return the signal value at ``time_seconds``."""


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject slippery inputs."""

    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")

    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")

    return converted


@dataclass(frozen=True)
class LinearSpeakerSignal:
    """Toy speaker model that scales an input signal into a pressure proxy."""

    analog_signal: AnalogSignal
    speaker_gain: float = DEFAULT_SPEAKER_GAIN

    def __post_init__(self) -> None:
        if not hasattr(self.analog_signal, "value_at"):
            raise ValueError("analog_signal must expose value_at(time_seconds)")
        object.__setattr__(
            self,
            "speaker_gain",
            _finite_float("speaker_gain", self.speaker_gain),
        )

    def value_at(self, time_seconds: Real) -> float:
        """Return a normalized pressure proxy for the supplied time."""

        time = _finite_float("time_seconds", time_seconds)
        return self.speaker_gain * self.analog_signal.value_at(time)
