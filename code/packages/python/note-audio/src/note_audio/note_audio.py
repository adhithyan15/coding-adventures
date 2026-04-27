"""Compose typed notes, oscillators, sampling, PCM, DAC, and speaker models.

`note-audio` is intentionally the teaching/orchestration layer. The reusable
stage behavior lives in smaller packages:

```text
note-frequency -> oscillator -> pcm-audio -> virtual-dac -> virtual-speaker
```

This package wires those boxes together and returns an inspectable chain instead
of hiding the journey behind a black-box `play()` call.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real

from note_frequency import Note, parse_note
from oscillator import (
    SampleBuffer,
    SineOscillator,
    UniformSampler,
    sample_count_for_duration,
)
from pcm_audio import (
    DEFAULT_BIT_DEPTH,
    DEFAULT_CHANNEL_COUNT,
    DEFAULT_FULL_SCALE_VOLTAGE,
    DEFAULT_SAMPLE_RATE_HZ,
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
    encode_sample_buffer,
    float_to_pcm16,
    samples_to_pcm_buffer,
)
from virtual_dac import ZeroOrderHoldDACSignal, pcm16_to_voltage
from virtual_speaker import (
    DEFAULT_SPEAKER_GAIN,
    AnalogSignal,
    LinearSpeakerSignal,
)
from wav_sink import OutputPath, to_wav_bytes, write_wav

DEFAULT_AMPLITUDE = 0.8
DEFAULT_MAX_SAMPLE_COUNT = 10_000_000

NoteInput = Note | str

__all__ = [
    "DEFAULT_AMPLITUDE",
    "DEFAULT_BIT_DEPTH",
    "DEFAULT_CHANNEL_COUNT",
    "DEFAULT_FULL_SCALE_VOLTAGE",
    "DEFAULT_MAX_SAMPLE_COUNT",
    "DEFAULT_SAMPLE_RATE_HZ",
    "DEFAULT_SPEAKER_GAIN",
    "AnalogSignal",
    "LinearSpeakerSignal",
    "NoteEvent",
    "NoteInput",
    "OutputPath",
    "PCM16_MAX",
    "PCM16_MIN",
    "PCMBuffer",
    "PCMFormat",
    "RenderedNote",
    "ZeroOrderHoldDACSignal",
    "encode_sample_buffer",
    "float_to_pcm16",
    "pcm16_to_voltage",
    "render_note_to_sound_chain",
    "samples_to_pcm_buffer",
    "to_wav_bytes",
    "write_wav",
]


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
