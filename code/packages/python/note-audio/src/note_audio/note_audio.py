"""Compose typed notes, oscillators, sampling, PCM, DAC, and speaker models.

This package intentionally keeps every box in the signal chain visible. A
caller can inspect the note frequency, the continuous oscillator, the floating
samples, the PCM integers, the virtual DAC voltage, and the virtual speaker
pressure proxy before choosing any real playback sink.
"""

from __future__ import annotations

import wave
from collections.abc import Iterable
from dataclasses import dataclass
from io import BytesIO
from math import floor, isfinite
from numbers import Integral, Real
from os import PathLike
from pathlib import Path
from typing import Protocol

from note_frequency import Note, parse_note
from oscillator import (
    SampleBuffer,
    SineOscillator,
    UniformSampler,
    sample_count_for_duration,
)

DEFAULT_SAMPLE_RATE_HZ = 44_100.0
DEFAULT_AMPLITUDE = 0.8
DEFAULT_BIT_DEPTH = 16
DEFAULT_CHANNEL_COUNT = 1
DEFAULT_FULL_SCALE_VOLTAGE = 1.0
DEFAULT_SPEAKER_GAIN = 1.0
DEFAULT_MAX_SAMPLE_COUNT = 10_000_000
INTEGER_TOLERANCE = 1e-9
PCM16_MIN = -32_768
PCM16_MAX = 32_767

NoteInput = Note | str
OutputPath = str | PathLike[str]


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


def _parse_note_input(note: NoteInput) -> Note:
    if isinstance(note, Note):
        return note
    if isinstance(note, str):
        return parse_note(note)
    raise ValueError(f"note must be a note string or Note, got {note!r}")


@dataclass(frozen=True)
class NoteEvent:
    """A human note label plus the timing and loudness needed to render it."""

    note: NoteInput
    duration_seconds: float
    amplitude: float = DEFAULT_AMPLITUDE
    start_time_seconds: float = 0.0

    def __post_init__(self) -> None:
        object.__setattr__(self, "note", _parse_note_input(self.note))
        object.__setattr__(
            self,
            "duration_seconds",
            _non_negative_float("duration_seconds", self.duration_seconds),
        )
        object.__setattr__(
            self,
            "amplitude",
            _non_negative_float("amplitude", self.amplitude),
        )
        object.__setattr__(
            self,
            "start_time_seconds",
            _finite_float("start_time_seconds", self.start_time_seconds),
        )


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


@dataclass(frozen=True)
class LinearSpeakerSignal:
    """Toy speaker model that turns DAC voltage into a pressure-like signal."""

    analog_signal: AnalogSignal
    speaker_gain: float = DEFAULT_SPEAKER_GAIN

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "speaker_gain",
            _finite_float("speaker_gain", self.speaker_gain),
        )

    def value_at(self, time_seconds: Real) -> float:
        """Return a normalized pressure proxy for the supplied time."""

        return self.speaker_gain * self.analog_signal.value_at(time_seconds)


@dataclass(frozen=True)
class RenderedNote:
    """The full inspectable chain from a note event to virtual speaker output."""

    note_event: NoteEvent
    frequency_hz: float
    oscillator: SineOscillator
    floating_samples: SampleBuffer
    pcm_buffer: PCMBuffer
    dac_signal: ZeroOrderHoldDACSignal
    speaker_signal: LinearSpeakerSignal

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "frequency_hz",
            _positive_float("frequency_hz", self.frequency_hz),
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


def pcm16_to_voltage(sample: int, pcm_format: PCMFormat | None = None) -> float:
    """Map a signed 16-bit PCM integer to virtual DAC voltage."""

    active_format = pcm_format if pcm_format is not None else PCMFormat()
    checked = PCMBuffer((sample,), active_format).samples[0]
    if checked >= 0:
        return checked / PCM16_MAX * active_format.full_scale_voltage
    return checked / abs(PCM16_MIN) * active_format.full_scale_voltage


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


def _validate_sample_budget(sample_count: int, max_sample_count: int) -> None:
    limit = _non_negative_int("max_sample_count", max_sample_count)
    if sample_count > limit:
        raise ValueError(
            f"render would create {sample_count} samples, "
            f"above max_sample_count={limit}"
        )


def render_note_to_sound_chain(
    note: NoteInput,
    duration_seconds: Real,
    *,
    sample_rate_hz: Real = DEFAULT_SAMPLE_RATE_HZ,
    amplitude: Real = DEFAULT_AMPLITUDE,
    phase_cycles: Real = 0.0,
    start_time_seconds: Real = 0.0,
    full_scale_voltage: Real = DEFAULT_FULL_SCALE_VOLTAGE,
    speaker_gain: Real = DEFAULT_SPEAKER_GAIN,
    max_sample_count: int = DEFAULT_MAX_SAMPLE_COUNT,
) -> RenderedNote:
    """Render a note while keeping every MUS01 layer available for inspection."""

    event = NoteEvent(
        note=note,
        duration_seconds=_non_negative_float("duration_seconds", duration_seconds),
        amplitude=_non_negative_float("amplitude", amplitude),
        start_time_seconds=_finite_float("start_time_seconds", start_time_seconds),
    )
    sample_rate = _positive_float("sample_rate_hz", sample_rate_hz)
    frequency = event.note.frequency()
    if frequency >= sample_rate / 2.0:
        raise ValueError(
            f"frequency_hz {frequency} must be below Nyquist {sample_rate / 2.0}"
        )

    count = sample_count_for_duration(event.duration_seconds, sample_rate)
    _validate_sample_budget(count, max_sample_count)

    oscillator = SineOscillator(
        frequency_hz=frequency,
        amplitude=event.amplitude,
        phase_cycles=_finite_float("phase_cycles", phase_cycles),
        offset=0.0,
    )
    floating_samples = UniformSampler(sample_rate).sample(
        oscillator,
        event.duration_seconds,
        event.start_time_seconds,
    )
    pcm_format = PCMFormat(
        sample_rate_hz=sample_rate,
        channel_count=DEFAULT_CHANNEL_COUNT,
        bit_depth=DEFAULT_BIT_DEPTH,
        full_scale_voltage=_positive_float("full_scale_voltage", full_scale_voltage),
    )
    pcm_buffer = encode_sample_buffer(floating_samples, pcm_format)
    dac_signal = ZeroOrderHoldDACSignal(pcm_buffer)
    speaker_signal = LinearSpeakerSignal(
        analog_signal=dac_signal,
        speaker_gain=_finite_float("speaker_gain", speaker_gain),
    )

    return RenderedNote(
        note_event=event,
        frequency_hz=frequency,
        oscillator=oscillator,
        floating_samples=floating_samples,
        pcm_buffer=pcm_buffer,
        dac_signal=dac_signal,
        speaker_signal=speaker_signal,
    )


def to_wav_bytes(pcm_buffer: PCMBuffer) -> bytes:
    """Write a mono signed-16-bit PCM buffer into a deterministic WAV container."""

    if not isinstance(pcm_buffer, PCMBuffer):
        raise ValueError("pcm_buffer must be a PCMBuffer")

    sample_rate = pcm_buffer.pcm_format.integer_sample_rate()
    output = BytesIO()
    with wave.open(output, "wb") as wav_file:
        wav_file.setnchannels(pcm_buffer.pcm_format.channel_count)
        wav_file.setsampwidth(pcm_buffer.pcm_format.sample_width_bytes)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(pcm_buffer.to_little_endian_bytes())
    return output.getvalue()


def write_wav(path: OutputPath, pcm_buffer: PCMBuffer) -> Path:
    """Write a WAV file and return the resolved ``Path`` object used."""

    output_path = Path(path)
    output_path.write_bytes(to_wav_bytes(pcm_buffer))
    return output_path


def samples_to_pcm_buffer(
    samples: Iterable[Real],
    *,
    sample_rate_hz: Real,
    start_time_seconds: Real = 0.0,
    pcm_format: PCMFormat | None = None,
) -> PCMBuffer:
    """Convenience helper for tests and demos that start with raw float samples."""

    sample_buffer = SampleBuffer(
        samples=tuple(float_to_pcm_input(sample) for sample in samples),
        sample_rate_hz=_positive_float("sample_rate_hz", sample_rate_hz),
        start_time_seconds=_finite_float("start_time_seconds", start_time_seconds),
    )
    return encode_sample_buffer(sample_buffer, pcm_format)


def float_to_pcm_input(sample: Real) -> float:
    """Validate a raw floating sample before putting it in a SampleBuffer."""

    return _finite_float("sample", sample)
