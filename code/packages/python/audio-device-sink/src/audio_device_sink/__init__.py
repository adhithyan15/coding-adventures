"""Rust-backed audio device sink for signed 16-bit PCM buffers."""

from __future__ import annotations

from collections.abc import Iterable
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
MAX_SAMPLE_RATE_HZ = 384_000
MAX_BLOCKING_DURATION_SECONDS = 10.0 * 60.0


@dataclass(frozen=True)
class PlaybackReport:
    """Summary returned after a sink accepts a buffer for blocking playback."""

    frames_played: int
    sample_rate_hz: int
    channel_count: int
    duration_seconds: float
    backend_name: str


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
        converted = float(value)
        if not isfinite(converted) or converted != round(converted):
            raise ValueError("sample_rate_hz must be an integer-valued rate")
        rate = int(round(converted))
    else:
        raise TypeError("sample_rate_hz must be an integer")

    if rate <= 0:
        raise ValueError("sample_rate_hz must be > 0")
    if rate > MAX_SAMPLE_RATE_HZ:
        raise ValueError(f"sample_rate_hz must be <= {MAX_SAMPLE_RATE_HZ}")
    return rate


def _channel_count(value: Any) -> int:
    count = _integer("channel_count", value)
    if count != 1:
        raise ValueError("only mono channel_count=1 is supported in V1")
    return count


def _normalize_samples(samples: Iterable[Any], sample_rate_hz: int) -> list[int]:
    max_samples = int(sample_rate_hz * MAX_BLOCKING_DURATION_SECONDS)
    normalized: list[int] = []
    for index, sample in enumerate(samples):
        value = _integer(f"samples[{index}]", sample)
        if value < PCM16_MIN or value > PCM16_MAX:
            raise ValueError(
                f"samples[{index}] must fit signed 16-bit PCM, got {value}"
            )
        normalized.append(value)
        if len(normalized) > max_samples:
            raise ValueError(
                f"blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
            )
    return normalized


def _report_from_native(native_report: tuple[int, int, int, float, str]) -> PlaybackReport:
    frames_played, sample_rate_hz, channel_count, duration_seconds, backend_name = (
        native_report
    )
    return PlaybackReport(
        frames_played=int(frames_played),
        sample_rate_hz=int(sample_rate_hz),
        channel_count=int(channel_count),
        duration_seconds=float(duration_seconds),
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

    return play_samples(
        samples,
        sample_rate_hz=sample_rate_hz,
        channel_count=channel_count,
    )


__version__ = "0.1.0"

__all__ = [
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
