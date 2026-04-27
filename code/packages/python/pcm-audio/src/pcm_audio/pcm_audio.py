"""Encode normalized floating samples into signed 16-bit PCM.

PCM is the first digital audio stage in the note-to-sound chain. It takes the
smooth world that the sampler observed and stores each value as a finite integer
that file formats, DACs, and device sinks can understand.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real

from oscillator import SampleBuffer

DEFAULT_SAMPLE_RATE_HZ = 44_100.0
DEFAULT_BIT_DEPTH = 16
DEFAULT_CHANNEL_COUNT = 1
DEFAULT_FULL_SCALE_VOLTAGE = 1.0
INTEGER_TOLERANCE = 1e-9
PCM16_MIN = -32_768
PCM16_MAX = 32_767


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject slippery inputs."""

    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")

    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")

    return converted


def _positive_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _non_negative_int(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer >= 0, got {value!r}")
    converted = int(value)
    if converted < 0:
        raise ValueError(f"{name} must be >= 0, got {converted}")
    return converted


def _positive_int(name: str, value: int) -> int:
    converted = _non_negative_int(name, value)
    if converted == 0:
        raise ValueError(f"{name} must be > 0, got 0")
    return converted


def _integer_sample_rate(sample_rate_hz: float) -> int:
    rounded = round(sample_rate_hz)
    if abs(sample_rate_hz - rounded) > INTEGER_TOLERANCE:
        raise ValueError(
            "sample_rate_hz must be an integer-valued rate for WAV output, "
            f"got {sample_rate_hz}"
        )
    return int(rounded)


@dataclass(frozen=True)
class PCMFormat:
    """Metadata for the first digital audio representation: mono 16-bit PCM."""

    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ
    channel_count: int = DEFAULT_CHANNEL_COUNT
    bit_depth: int = DEFAULT_BIT_DEPTH
    full_scale_voltage: float = DEFAULT_FULL_SCALE_VOLTAGE

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "sample_rate_hz",
            _positive_float("sample_rate_hz", self.sample_rate_hz),
        )
        channel_count = _positive_int("channel_count", self.channel_count)
        if channel_count != DEFAULT_CHANNEL_COUNT:
            raise ValueError("only mono PCM is supported in V1")
        object.__setattr__(self, "channel_count", channel_count)

        bit_depth = _positive_int("bit_depth", self.bit_depth)
        if bit_depth != DEFAULT_BIT_DEPTH:
            raise ValueError("only signed 16-bit PCM is supported in V1")
        object.__setattr__(self, "bit_depth", bit_depth)
        object.__setattr__(
            self,
            "full_scale_voltage",
            _positive_float("full_scale_voltage", self.full_scale_voltage),
        )

    @property
    def minimum_integer(self) -> int:
        """Return the lowest signed PCM integer for this format."""

        return PCM16_MIN

    @property
    def maximum_integer(self) -> int:
        """Return the highest signed PCM integer for this format."""

        return PCM16_MAX

    @property
    def sample_width_bytes(self) -> int:
        """Return the number of bytes per PCM sample."""

        return self.bit_depth // 8

    def integer_sample_rate(self) -> int:
        """Return a WAV-compatible integer sample rate."""

        return _integer_sample_rate(self.sample_rate_hz)


@dataclass(frozen=True)
class PCMBuffer:
    """PCM samples plus the timing metadata needed by DAC and file sinks."""

    samples: tuple[int, ...]
    pcm_format: PCMFormat
    start_time_seconds: float = 0.0
    clipped_sample_count: int = 0

    def __post_init__(self) -> None:
        checked_samples: list[int] = []
        for index, sample in enumerate(self.samples):
            if isinstance(sample, bool) or not isinstance(sample, Integral):
                raise ValueError(f"samples[{index}] must be a PCM integer")
            converted = int(sample)
            if converted < PCM16_MIN or converted > PCM16_MAX:
                raise ValueError(
                    f"samples[{index}] must fit signed 16-bit PCM, got {converted}"
                )
            checked_samples.append(converted)

        object.__setattr__(self, "samples", tuple(checked_samples))
        if not isinstance(self.pcm_format, PCMFormat):
            raise ValueError("pcm_format must be a PCMFormat")
        object.__setattr__(
            self,
            "start_time_seconds",
            _finite_float("start_time_seconds", self.start_time_seconds),
        )
        object.__setattr__(
            self,
            "clipped_sample_count",
            _non_negative_int("clipped_sample_count", self.clipped_sample_count),
        )

    def sample_count(self) -> int:
        """Return the number of PCM samples."""

        return len(self.samples)

    def sample_period_seconds(self) -> float:
        """Return the time spacing between adjacent samples."""

        return 1.0 / self.pcm_format.sample_rate_hz

    def duration_seconds(self) -> float:
        """Return the half-open duration covered by this PCM buffer."""

        return self.sample_count() / self.pcm_format.sample_rate_hz

    def time_at(self, index: int) -> float:
        """Return the timestamp for a PCM sample index."""

        if isinstance(index, bool) or not isinstance(index, Integral):
            raise ValueError(f"index must be an integer, got {index!r}")
        converted = int(index)
        if not 0 <= converted < self.sample_count():
            raise ValueError(
                f"index must be in [0, {self.sample_count()}), got {converted}"
            )
        return self.start_time_seconds + converted / self.pcm_format.sample_rate_hz

    def to_little_endian_bytes(self) -> bytes:
        """Pack signed 16-bit PCM integers as little-endian bytes."""

        return b"".join(
            sample.to_bytes(2, "little", signed=True) for sample in self.samples
        )


def float_to_pcm16(sample: Real) -> tuple[int, bool]:
    """Convert one normalized floating sample to signed 16-bit PCM.

    The boolean return value tells the caller whether clipping was required.
    """

    value = _finite_float("sample", sample)
    clipped = value < -1.0 or value > 1.0
    bounded = min(1.0, max(-1.0, value))
    if bounded >= 0.0:
        return int(round(bounded * PCM16_MAX)), clipped
    return int(round(bounded * abs(PCM16_MIN))), clipped


def encode_sample_buffer(
    sample_buffer: SampleBuffer,
    pcm_format: PCMFormat | None = None,
) -> PCMBuffer:
    """Quantize floating oscillator samples into signed 16-bit PCM."""

    if not isinstance(sample_buffer, SampleBuffer):
        raise ValueError("sample_buffer must be an oscillator.SampleBuffer")

    active_format = pcm_format if pcm_format is not None else PCMFormat(
        sample_rate_hz=sample_buffer.sample_rate_hz
    )
    pcm_samples: list[int] = []
    clipped_count = 0
    for sample in sample_buffer.samples:
        pcm_sample, clipped = float_to_pcm16(sample)
        pcm_samples.append(pcm_sample)
        if clipped:
            clipped_count += 1

    return PCMBuffer(
        samples=tuple(pcm_samples),
        pcm_format=active_format,
        start_time_seconds=sample_buffer.start_time_seconds,
        clipped_sample_count=clipped_count,
    )


def samples_to_pcm_buffer(
    samples: Iterable[Real],
    *,
    sample_rate_hz: Real,
    start_time_seconds: Real = 0.0,
    pcm_format: PCMFormat | None = None,
) -> PCMBuffer:
    """Convenience helper for demos that start with raw floating samples."""

    sample_buffer = SampleBuffer(
        samples=tuple(_finite_float("sample", sample) for sample in samples),
        sample_rate_hz=_positive_float("sample_rate_hz", sample_rate_hz),
        start_time_seconds=_finite_float("start_time_seconds", start_time_seconds),
    )
    return encode_sample_buffer(sample_buffer, pcm_format)
