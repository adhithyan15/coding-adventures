"""
Virtual oscillators and samplers.

An oscillator in this package is not a real sound card, timer, or circuit. It
is a mathematical signal: give it a time in seconds and it returns the value the
signal would have at that instant.

A sampler sits one layer above that. It chooses concrete times, asks the signal
for values, and stores those values in a `SampleBuffer`. That is the bridge from
a smooth virtual signal to the stream of numbers that later audio, radio, DAC,
or clock-edge packages can consume.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator, Sized
from dataclasses import dataclass
from math import floor, isfinite
from numbers import Integral, Real
from typing import Protocol

from trig import PI, sin

TWO_PI: float = 2.0 * PI
INTEGER_TOLERANCE: float = 1e-9

# CWE-400 resource exhaustion defense bounds
MAX_SAMPLE_RATE_HZ: float = 192_000.0
MAX_DURATION_SECONDS: float = 60.0
MAX_SAMPLES: int = 16_777_216


class ContinuousSignal(Protocol):
    """Anything that can report its value at a time in seconds."""

    def value_at(self, time_seconds: float) -> float:
        """Return the signal value at `time_seconds`."""


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
    """
    Convert a real number to `float` and reject non-finite values.

    The spec talks about "finite real numbers". Python makes that slightly
    slippery because `float("nan")`, infinities, strings, and booleans can sneak
    into numeric-looking APIs. Centralizing the check keeps every public
    constructor consistent.
    """
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


def _non_negative_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted < 0.0:
        raise ValueError(f"{name} must be >= 0.0, got {converted}")
    return converted


def _positive_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _sample_rate(name: str, value: Real) -> float:
    converted = _positive_float(name, value)
    if converted > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"{name} {converted} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )
    if not isfinite(1.0 / converted):
        raise ValueError(f"{name} subnormal {converted} has non-finite period")
    return converted


def _duration(name: str, value: Real) -> float:
    converted = _non_negative_float(name, value)
    if converted > MAX_DURATION_SECONDS:
        raise ValueError(
            f"{name} {converted} exceeds MAX_DURATION_SECONDS={MAX_DURATION_SECONDS}"
        )
    return converted


def _fractional_part(value: float) -> float:
    """
    Return the fractional part in the half-open interval [0.0, 1.0).

    Python's `%` would also work for positive divisors, but `x - floor(x)`
    mirrors the spec directly and makes the negative-time behavior obvious.
    """
    if not isfinite(value):
        return float("nan")
    return value - floor(value)


def nyquist_frequency(sample_rate_hz: Real) -> float:
    """Return half the sample rate, the highest cleanly representable frequency."""
    return _sample_rate("sample_rate_hz", sample_rate_hz) / 2.0


def sample_count_for_duration(duration_seconds: Real, sample_rate_hz: Real) -> int:
    """
    Return the default sample count for a half-open sampling interval.

    Mathematically this is `floor(duration_seconds * sample_rate_hz)`. The tiny
    integer tolerance prevents floating-point spelling accidents such as
    `479.99999999999994` becoming 479 when the intended mathematical product is
    480.
    """
    duration = _duration("duration_seconds", duration_seconds)
    sample_rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    raw_count = duration * sample_rate
    nearest_integer = round(raw_count)

    if abs(raw_count - nearest_integer) <= INTEGER_TOLERANCE:
        count = int(nearest_integer)
    else:
        count = int(floor(raw_count))

    if count > MAX_SAMPLES:
        raise ValueError(f"sample count {count} exceeds MAX_SAMPLES={MAX_SAMPLES}")
    return count


def time_at_sample(
    index: int,
    sample_rate_hz: Real,
    start_time_seconds: Real = 0.0,
) -> float:
    """Return the time for sample index `index` on a uniform sample grid."""
    if isinstance(index, bool) or not isinstance(index, Integral):
        raise ValueError(f"index must be an integer >= 0, got {_safe_repr(index)}")
    idx = int(index)
    if idx < 0:
        raise ValueError(f"index must be >= 0, got {_safe_repr(idx)}")
    if idx >= MAX_SAMPLES:
        raise ValueError(f"index {_safe_repr(idx)} exceeds MAX_SAMPLES={MAX_SAMPLES}")

    sample_rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    start_time = _finite_float("start_time_seconds", start_time_seconds)
    result = start_time + idx / sample_rate
    if not isfinite(result):
        raise ValueError("computed sample time is non-finite")
    return result


@dataclass(frozen=True)
class SineOscillator:
    """
    Smooth sine oscillator.

    A 1 Hz default sine oscillator starts at zero, reaches its peak at 0.25
    seconds, crosses zero again at 0.5 seconds, and reaches its trough at 0.75
    seconds. Increasing the frequency squeezes more cycles into each second.
    """

    frequency_hz: float
    amplitude: float = 1.0
    phase_cycles: float = 0.0
    offset: float = 0.0

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "frequency_hz",
            _non_negative_float("frequency_hz", self.frequency_hz),
        )
        object.__setattr__(
            self,
            "amplitude",
            _non_negative_float("amplitude", self.amplitude),
        )
        object.__setattr__(
            self,
            "phase_cycles",
            _finite_float("phase_cycles", self.phase_cycles),
        )
        object.__setattr__(self, "offset", _finite_float("offset", self.offset))

    def value_at(self, time_seconds: float) -> float:
        """Evaluate `offset + amplitude * sin(2*pi*(f*t + phase))`."""
        time = _finite_float("time_seconds", time_seconds)
        phase = self.frequency_hz * time + self.phase_cycles
        # trig.sin is defined for |x| <= 2**29 radians and returns NaN beyond it
        # (PHY00a 3.4.1).  A phase kept in cycles grows without bound (a 440 Hz
        # tone passes 2**29 radians after about 54 hours), so reduce it to
        # [0, 1) cycles first.  `phase - floor(phase)` is exact in binary64, and
        # it is also more accurate than multiplying a huge phase by 2*pi.
        # A non-finite phase (frequency * time overflowed) gives NaN, as in the Rust
        # and TypeScript lanes; math.floor would raise OverflowError instead.
        cycles = phase - floor(phase) if isfinite(phase) else float("nan")
        return self.offset + self.amplitude * sin(TWO_PI * cycles)


@dataclass(frozen=True)
class SquareOscillator:
    """
    High/low oscillator with a configurable duty cycle.

    The square oscillator is what lets the same oscillator abstraction support
    digital-looking signals. A clock package can hide this inside its own API
    and expose friendly `ClockEdge` records to consumers.
    """

    frequency_hz: float
    low: float = -1.0
    high: float = 1.0
    duty_cycle: float = 0.5
    phase_cycles: float = 0.0

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "frequency_hz",
            _non_negative_float("frequency_hz", self.frequency_hz),
        )
        object.__setattr__(self, "low", _finite_float("low", self.low))
        object.__setattr__(self, "high", _finite_float("high", self.high))
        object.__setattr__(
            self,
            "duty_cycle",
            _finite_float("duty_cycle", self.duty_cycle),
        )
        object.__setattr__(
            self,
            "phase_cycles",
            _finite_float("phase_cycles", self.phase_cycles),
        )

        if not 0.0 < self.duty_cycle < 1.0:
            raise ValueError(
                f"duty_cycle must satisfy 0.0 < duty_cycle < 1.0, got {self.duty_cycle}"
            )

    def value_at(self, time_seconds: float) -> float:
        """Return `high` during the duty-cycle window, otherwise `low`."""
        time = _finite_float("time_seconds", time_seconds)
        position = _fractional_part(self.frequency_hz * time + self.phase_cycles)
        if position < self.duty_cycle:
            return self.high
        return self.low


@dataclass(frozen=True)
class SampleBuffer:
    """
    Samples plus enough timing metadata to interpret them.

    The buffer does not know whether its samples represent audio, radio, a
    virtual voltage, or something else. It only knows the values and the uniform
    time grid they came from.
    """

    samples: tuple[float, ...]
    sample_rate_hz: float
    start_time_seconds: float = 0.0

    def __post_init__(self) -> None:
        if isinstance(self.samples, (str, bytes, bytearray)) or not isinstance(
            self.samples, Iterable
        ):
            raise ValueError(
                f"samples must be an iterable of real numbers, "
                f"got {_safe_repr(self.samples)}"
            )
        if isinstance(self.samples, Sized):
            try:
                declared_len = len(self.samples)
            except OverflowError as exc:
                raise ValueError(
                    f"sample count exceeds MAX_SAMPLES={MAX_SAMPLES}"
                ) from exc
            if declared_len > MAX_SAMPLES:
                raise ValueError(
                    f"sample count {declared_len} exceeds MAX_SAMPLES={MAX_SAMPLES}"
                )
        converted_list: list[float] = []
        for index, sample in enumerate(self.samples):
            if index >= MAX_SAMPLES:
                raise ValueError(f"sample count exceeds MAX_SAMPLES={MAX_SAMPLES}")
            converted_list.append(_finite_float(f"samples[{index}]", sample))
        converted_samples = tuple(converted_list)
        object.__setattr__(self, "samples", converted_samples)
        object.__setattr__(
            self,
            "sample_rate_hz",
            _sample_rate("sample_rate_hz", self.sample_rate_hz),
        )
        object.__setattr__(
            self,
            "start_time_seconds",
            _finite_float("start_time_seconds", self.start_time_seconds),
        )

    def sample_count(self) -> int:
        """Return the number of stored samples."""
        return len(self.samples)

    def sample_period_seconds(self) -> float:
        """Return the spacing between adjacent samples."""
        return 1.0 / self.sample_rate_hz

    def duration_seconds(self) -> float:
        """Return the covered duration of this half-open sample buffer."""
        return self.sample_count() / self.sample_rate_hz

    def time_at(self, index: int) -> float:
        """Return the time associated with a stored sample index."""
        if isinstance(index, bool) or not isinstance(index, Integral):
            raise ValueError(f"index must be an integer, got {_safe_repr(index)}")
        idx = int(index)
        if not 0 <= idx < self.sample_count():
            raise ValueError(
                f"index must be in [0, {self.sample_count()}), got {_safe_repr(idx)}"
            )

        return time_at_sample(
            idx,
            self.sample_rate_hz,
            self.start_time_seconds,
        )


@dataclass(frozen=True)
class UniformSampler:
    """Sampler that evaluates a signal at evenly spaced times."""

    sample_rate_hz: float

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "sample_rate_hz",
            _sample_rate("sample_rate_hz", self.sample_rate_hz),
        )

    def sample(
        self,
        signal: ContinuousSignal,
        duration_seconds: Real,
        start_time_seconds: Real = 0.0,
    ) -> SampleBuffer:
        """Sample `signal` over `[start_time_seconds, end_time_seconds)`."""
        count = sample_count_for_duration(duration_seconds, self.sample_rate_hz)
        return self.sample_count(signal, count, start_time_seconds)

    def sample_count(
        self,
        signal: ContinuousSignal,
        sample_count: int,
        start_time_seconds: Real = 0.0,
    ) -> SampleBuffer:
        """Sample exactly `sample_count` values from `signal`."""
        samples = tuple(self.samples(signal, sample_count, start_time_seconds))
        start_time = _finite_float("start_time_seconds", start_time_seconds)
        return SampleBuffer(
            samples=samples,
            sample_rate_hz=self.sample_rate_hz,
            start_time_seconds=start_time,
        )

    def samples(
        self,
        signal: ContinuousSignal,
        sample_count: int,
        start_time_seconds: Real = 0.0,
    ) -> Iterator[float]:
        """Yield the same values that `sample_count(...)` would store."""
        if not hasattr(signal, "value_at") or not callable(signal.value_at):
            raise ValueError("signal must expose callable value_at(time_seconds)")
        if isinstance(sample_count, bool) or not isinstance(sample_count, Integral):
            raise ValueError(
                f"sample_count must be an integer >= 0, got {_safe_repr(sample_count)}"
            )
        count = int(sample_count)
        if count < 0:
            raise ValueError(f"sample_count must be >= 0, got {_safe_repr(count)}")
        if count > MAX_SAMPLES:
            raise ValueError(
                f"sample_count {_safe_repr(count)} exceeds MAX_SAMPLES={MAX_SAMPLES}"
            )

        start_time = _finite_float("start_time_seconds", start_time_seconds)

        def _iter_samples() -> Iterator[float]:
            for index in range(count):
                yield signal.value_at(
                    time_at_sample(index, self.sample_rate_hz, start_time)
                )

        return _iter_samples()

    def nyquist_frequency(self) -> float:
        """Return this sampler's Nyquist frequency."""
        return self.sample_rate_hz / 2.0


__all__ = [
    "INTEGER_TOLERANCE",
    "MAX_DURATION_SECONDS",
    "MAX_SAMPLES",
    "MAX_SAMPLE_RATE_HZ",
    "TWO_PI",
    "ContinuousSignal",
    "SampleBuffer",
    "SineOscillator",
    "SquareOscillator",
    "UniformSampler",
    "nyquist_frequency",
    "sample_count_for_duration",
    "time_at_sample",
]
