"""Turn PCM integers into a virtual analog voltage signal.

The first DAC model is intentionally simple: a zero-order hold. Each PCM integer
becomes a voltage and that voltage is held until the next sample time.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import floor, isfinite
from numbers import Real

from pcm_audio import PCM16_MAX, PCM16_MIN, PCMBuffer, PCMFormat


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject slippery inputs."""

    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")

    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")

    return converted


def pcm16_to_voltage(sample: int, pcm_format: PCMFormat | None = None) -> float:
    """Map a signed 16-bit PCM integer to virtual DAC voltage."""

    active_format = pcm_format if pcm_format is not None else PCMFormat()
    checked = PCMBuffer((sample,), active_format).samples[0]
    if checked >= 0:
        return checked / PCM16_MAX * active_format.full_scale_voltage
    return checked / abs(PCM16_MIN) * active_format.full_scale_voltage


@dataclass(frozen=True)
class ZeroOrderHoldDACSignal:
    """Virtual DAC output using a simple zero-order hold staircase."""

    pcm_buffer: PCMBuffer

    def __post_init__(self) -> None:
        if not isinstance(self.pcm_buffer, PCMBuffer):
            raise ValueError("pcm_buffer must be a PCMBuffer")

    def value_at(self, time_seconds: Real) -> float:
        """Return the held DAC voltage at ``time_seconds``."""

        time = _finite_float("time_seconds", time_seconds)
        start = self.pcm_buffer.start_time_seconds
        end = start + self.pcm_buffer.duration_seconds()
        if time < start or time >= end or self.pcm_buffer.sample_count() == 0:
            return 0.0

        index = int(floor((time - start) * self.pcm_buffer.pcm_format.sample_rate_hz))
        if index < 0 or index >= self.pcm_buffer.sample_count():
            return 0.0
        return pcm16_to_voltage(
            self.pcm_buffer.samples[index],
            self.pcm_buffer.pcm_format,
        )
