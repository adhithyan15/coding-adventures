"""Rust-backed audio device sink for signed 16-bit PCM buffers."""

from __future__ import annotations

from collections.abc import Iterable, Sized
from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real
from typing import Any

from audio_device_sink.audio_device_sink import (  # type: ignore[import]
    AudioDeviceError,
    _play_samples,
)

PCM16_MIN = -32_768
PCM16_MAX = 32_767
MIN_SAMPLE_RATE_HZ = 1
MAX_SAMPLE_RATE_HZ = 384_000
MAX_CHANNELS = 1
MAX_BLOCKING_DURATION_SECONDS_INT = 10 * 60
MAX_BLOCKING_DURATION_SECONDS = 10.0 * 60.0
MAX_SAMPLES = MAX_SAMPLE_RATE_HZ * MAX_BLOCKING_DURATION_SECONDS_INT

_ERROR_INT_PREVIEW_LIMIT = 1_000_000


@dataclass(frozen=True)
class PlaybackReport:
    """Summary returned after a sink accepts a buffer for blocking playback."""

    frames_played: int
    sample_rate_hz: int
    channel_count: int
    duration_seconds: float
    backend_name: str


def _format_bounded_int(value: int) -> str:
    """Format an integer in O(1) without hitting Python's int_max_str_digits limit."""
    if -_ERROR_INT_PREVIEW_LIMIT <= value <= _ERROR_INT_PREVIEW_LIMIT:
        return str(value)
    return "out-of-range integer"


def _integer(name: str, value: Any) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise TypeError(f"{name} must be an integer")
    return int(value)


def _integer_sample_rate(value: Any) -> int:
    if isinstance(value, bool):
        raise TypeError("sample_rate_hz must be an integer")
    if isinstance(value, Integral):
        rate = int(value)
    elif isinstance(value, Real):
        try:
            converted = float(value)
        except OverflowError as exc:
            raise ValueError("sample_rate_hz is out of range") from exc
        if (
            not isfinite(converted)
            or converted != round(converted)
            or value != round(converted)
        ):
            raise ValueError("sample_rate_hz must be an integer-valued rate")
        rate = round(converted)
    else:
        raise TypeError("sample_rate_hz must be an integer")

    if rate < MIN_SAMPLE_RATE_HZ:
        raise ValueError("sample_rate_hz must be > 0")
    if rate > MAX_SAMPLE_RATE_HZ:
        raise ValueError(f"sample_rate_hz must be <= {MAX_SAMPLE_RATE_HZ}")
    return rate


def _channel_count(value: Any) -> int:
    count = _integer("channel_count", value)
    if count != MAX_CHANNELS:
        raise ValueError("only mono channel_count=1 is supported in V1")
    return count


def _blocking_limit_error() -> ValueError:
    return ValueError(
        f"blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
    )


def _normalize_samples(samples: Iterable[Any], sample_rate_hz: int) -> list[int]:
    if isinstance(samples, (str, bytes, bytearray)):
        raise TypeError("samples must be an iterable of signed 16-bit PCM integers")

    max_samples = min(
        sample_rate_hz * MAX_BLOCKING_DURATION_SECONDS_INT,
        MAX_SAMPLES,
    )

    if isinstance(samples, Sized):
        try:
            sample_len = len(samples)
        except OverflowError as exc:
            raise _blocking_limit_error() from exc
        if sample_len > max_samples:
            raise _blocking_limit_error()

    normalized: list[int] = []
    for index, sample in enumerate(samples):
        if index >= max_samples:
            raise _blocking_limit_error()
        value = _integer(f"samples[{index}]", sample)
        if value < PCM16_MIN or value > PCM16_MAX:
            bounded = _format_bounded_int(value)
            raise ValueError(
                f"samples[{index}] must fit signed 16-bit PCM, got {bounded}"
            )
        normalized.append(value)
    return normalized


def _report_from_native(
    native_report: tuple[int, int, int, float, str],
) -> PlaybackReport:
    frames_played, sample_rate_hz, channel_count, duration_seconds, backend_name = (
        native_report
    )
    dur = float(duration_seconds)
    if not isfinite(dur) or dur < 0.0:
        raise AudioDeviceError("native backend returned a non-finite playback duration")
    return PlaybackReport(
        frames_played=int(frames_played),
        sample_rate_hz=int(sample_rate_hz),
        channel_count=int(channel_count),
        duration_seconds=dur,
        backend_name=str(backend_name),
    )


def play_samples(
    samples: Iterable[Any],
    *,
    sample_rate_hz: Any,
    channel_count: Any = 1,
) -> PlaybackReport:
    """Play raw signed 16-bit mono PCM samples through the default device."""

    rate = _integer_sample_rate(sample_rate_hz)
    channels = _channel_count(channel_count)
    normalized = _normalize_samples(samples, rate)
    return _report_from_native(_play_samples(normalized, rate, channels))


def play_pcm_buffer(buffer: Any) -> PlaybackReport:
    """Adapt a ``pcm_audio.PCMBuffer``-like object into the native sink."""

    try:
        samples = buffer.samples
        pcm_format = buffer.pcm_format
        sample_rate_hz = pcm_format.sample_rate_hz
        channel_count = pcm_format.channel_count
    except AttributeError as exc:
        raise TypeError("buffer must look like pcm_audio.PCMBuffer") from exc

    if hasattr(pcm_format, "bit_depth"):
        bit_depth = _integer("bit_depth", pcm_format.bit_depth)
        if bit_depth != 16:
            raise ValueError("only signed 16-bit PCM is supported in V1")

    return play_samples(
        samples,
        sample_rate_hz=sample_rate_hz,
        channel_count=channel_count,
    )


__version__ = "0.1.0"

__all__ = [  # noqa: RUF022 - preserve the package's stable export order
    "AudioDeviceError",
    "MAX_BLOCKING_DURATION_SECONDS",
    "MAX_SAMPLE_RATE_HZ",
    "PCM16_MAX",
    "PCM16_MIN",
    "PlaybackReport",
    "__version__",
    "play_pcm_buffer",
    "play_samples",
]
