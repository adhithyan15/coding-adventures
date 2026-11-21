"""Turn PCM integers into a virtual analog voltage signal.

The first DAC model is intentionally simple: a zero-order hold. Each PCM integer
becomes a voltage and that voltage is held until the next sample time.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import floor, isfinite
from numbers import Integral, Real

from pcm_audio import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
)

__all__ = [
    "MAX_DURATION_SECONDS",
    "MAX_SAMPLES",
    "MAX_SAMPLE_RATE_HZ",
    "ZeroOrderHoldDACSignal",
    "pcm16_to_voltage",
]


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


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject slippery inputs."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(
            f"{name} must be a finite real number, got {_safe_repr(value)}"
        )

    try:
        converted = float(value)
    except OverflowError as exc:
        raise ValueError(
            f"{name} overflows float range: {_safe_repr(value)}"
        ) from exc
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {_safe_repr(value)}")

    return converted


def pcm16_to_voltage(sample: int, pcm_format: PCMFormat | None = None) -> float:
    """Map a signed 16-bit PCM integer to virtual DAC voltage."""
    if isinstance(sample, bool) or not isinstance(sample, Integral):
        raise ValueError(f"sample must be a PCM integer, got {_safe_repr(sample)}")

    s = int(sample)
    if s < PCM16_MIN or s > PCM16_MAX:
        raise ValueError(
            f"sample {_safe_repr(s)} must fit signed 16-bit PCM "
            f"[{PCM16_MIN}, {PCM16_MAX}]"
        )

    active_format = pcm_format if pcm_format is not None else PCMFormat()
    if not isinstance(active_format, PCMFormat):
        raise ValueError("pcm_format must be a PCMFormat")

    if s >= 0:
        return s / PCM16_MAX * active_format.full_scale_voltage
    return s / abs(PCM16_MIN) * active_format.full_scale_voltage


@dataclass(frozen=True)
class ZeroOrderHoldDACSignal:
    """Virtual DAC output using a simple zero-order hold staircase."""

    pcm_buffer: PCMBuffer

    def __post_init__(self) -> None:
        if not isinstance(self.pcm_buffer, PCMBuffer):
            raise ValueError("pcm_buffer must be a PCMBuffer")
        if self.pcm_buffer.sample_count() > MAX_SAMPLES:
            raise ValueError(
                f"pcm_buffer sample count {self.pcm_buffer.sample_count()} "
                f"exceeds MAX_SAMPLES={MAX_SAMPLES}"
            )
        if self.pcm_buffer.pcm_format.sample_rate_hz > MAX_SAMPLE_RATE_HZ:
            raise ValueError(
                f"pcm_buffer sample rate {self.pcm_buffer.pcm_format.sample_rate_hz} "
                f"exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
            )

    def value_at(self, time_seconds: Real) -> float:
        """Return the held DAC voltage at ``time_seconds``."""
        time = _finite_float("time_seconds", time_seconds)
        start = self.pcm_buffer.start_time_seconds
        end = start + self.pcm_buffer.duration_seconds()
        count = self.pcm_buffer.sample_count()
        if time < start or time >= end or count == 0:
            return 0.0

        scaled = (time - start) * self.pcm_buffer.pcm_format.sample_rate_hz
        if not isfinite(scaled):
            return 0.0
        index = int(floor(scaled))
        index = min(max(index, 0), count - 1)
        return pcm16_to_voltage(
            self.pcm_buffer.samples[index],
            self.pcm_buffer.pcm_format,
        )
