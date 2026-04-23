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

from collections.abc import Iterator
from dataclasses import dataclass
from math import floor, isfinite
from numbers import Integral, Real
from typing import Protocol

from trig import PI, sin

TWO_PI = 2.0 * PI
INTEGER_TOLERANCE = 1e-9


class ContinuousSignal(Protocol):
    """Anything that can report its value at a time in seconds."""

    def value_at(self, time_seconds: float) -> float:
        """Return the signal value at `time_seconds`."""


def _finite_float(name: str, value: Real) -> float:
    """
    Convert a real number to `float` and reject non-finite values.

    The spec talks about "finite real numbers". Python makes that slightly
    slippery because `float("nan")`, infinities, strings, and booleans can sneak
    into numeric-looking APIs. Centralizing the check keeps every public
    constructor consistent.
    """

    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")

    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")

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


def _fractional_part(value: float) -> float:
    """
    Return the fractional part in the half-open interval [0.0, 1.0).

    Python's `%` would also work for positive divisors, but `x - floor(x)`
    mirrors the spec directly and makes the negative-time behavior obvious.
    """

    return value - floor(value)


def nyquist_frequency(sample_rate_hz: Real) -> float:
    """Return half the sample rate, the highest cleanly representable frequency."""

    return _positive_float("sample_rate_hz", sample_rate_hz) / 2.0


def sample_count_for_duration(duration_seconds: Real, sample_rate_hz: Real) -> int:
    """
    Return the default sample count for a half-open sampling interval.

    Mathematically this is `floor(duration_seconds * sample_rate_hz)`. The tiny
    integer tolerance prevents floating-point spelling accidents such as
    `479.99999999999994` becoming 479 when the intended mathematical product is
    480.
    """

    duration = _non_negative_float("duration_seconds", duration_seconds)
    sample_rate = _positive_float("sample_rate_hz", sample_rate_hz)
    raw_count = duration * sample_rate
    nearest_integer = round(raw_count)

    if abs(raw_count - nearest_integer) <= INTEGER_TOLERANCE:
        return int(nearest_integer)

    return int(floor(raw_count))


def time_at_sample(
    index: int,
    sample_rate_hz: Real,
    start_time_seconds: Real = 0.0,
) -> float:
    """Return the time for sample index `index` on a uniform sample grid."""

    if isinstance(index, bool) or not isinstance(index, Integral):
        raise ValueError(f"index must be an integer >= 0, got {index!r}")
    if index < 0:
        raise ValueError(f"index must be >= 0, got {index}")

    sample_rate = _positive_float("sample_rate_hz", sample_rate_hz)
    start_time = _finite_float("start_time_seconds", start_time_seconds)
    return start_time + int(index) / sample_rate


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
        return self.offset + self.amplitude * sin(TWO_PI * phase)


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
                f"duty_cycle must satisfy 0.0 < duty_cycle < 1.0, "
                f"got {self.duty_cycle}"
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
        converted_samples = tuple(
            _finite_float(f"samples[{index}]", sample)
            for index, sample in enumerate(self.samples)
        )
        object.__setattr__(self, "samples", converted_samples)
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_float("sample_rate_hz", self.sample_rate_hz),
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
            raise ValueError(f"index must be an integer, got {index!r}")
        if not 0 <= index < self.sample_count():
            raise ValueError(
                f"index must be in [0, {self.sample_count()}), got {index}"
            )

        return time_at_sample(
            int(index),
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
            _positive_float("sample_rate_hz", self.sample_rate_hz),
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

        if isinstance(sample_count, bool) or not isinstance(sample_count, Integral):
            raise ValueError(
                f"sample_count must be an integer >= 0, got {sample_count!r}"
            )
        if sample_count < 0:
            raise ValueError(f"sample_count must be >= 0, got {sample_count}")

        start_time = _finite_float("start_time_seconds", start_time_seconds)
        for index in range(int(sample_count)):
            yield signal.value_at(
                time_at_sample(index, self.sample_rate_hz, start_time)
            )

    def nyquist_frequency(self) -> float:
        """Return this sampler's Nyquist frequency."""

        return nyquist_frequency(self.sample_rate_hz)
