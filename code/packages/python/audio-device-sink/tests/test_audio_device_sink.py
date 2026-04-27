from __future__ import annotations

import sys

import pytest
from pcm_audio import PCMBuffer, PCMFormat

import audio_device_sink as audio_device_sink_module
from audio_device_sink import (
    AudioDeviceError,
    PlaybackReport,
    play_pcm_buffer,
    play_samples,
)


def test_play_samples_normalizes_and_delegates_to_native_boundary(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

    def fake_play_samples(samples: list[int], sample_rate_hz: int, channel_count: int):
        captured.update(
            samples=samples,
            sample_rate_hz=sample_rate_hz,
            channel_count=channel_count,
        )
        return (3, sample_rate_hz, channel_count, 3 / sample_rate_hz, "fake")

    monkeypatch.setattr(audio_device_sink_module, "_play_samples", fake_play_samples)

    report = play_samples([0, 100, -100], sample_rate_hz=44_100.0)

    assert captured == {
        "samples": [0, 100, -100],
        "sample_rate_hz": 44_100,
        "channel_count": 1,
    }
    assert report == PlaybackReport(
        frames_played=3,
        sample_rate_hz=44_100,
        channel_count=1,
        duration_seconds=3 / 44_100,
        backend_name="fake",
    )


def test_play_pcm_buffer_adapts_existing_pcm_buffer(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    captured: dict[str, object] = {}

    def fake_play_samples(samples: list[int], sample_rate_hz: int, channel_count: int):
        captured.update(
            samples=samples,
            sample_rate_hz=sample_rate_hz,
            channel_count=channel_count,
        )
        return (2, sample_rate_hz, channel_count, 2 / sample_rate_hz, "fake")

    monkeypatch.setattr(audio_device_sink_module, "_play_samples", fake_play_samples)
    buffer = PCMBuffer(
        samples=(0, 32767),
        pcm_format=PCMFormat(sample_rate_hz=22_050),
    )

    report = play_pcm_buffer(buffer)

    assert captured == {
        "samples": [0, 32767],
        "sample_rate_hz": 22_050,
        "channel_count": 1,
    }
    assert report.frames_played == 2


@pytest.mark.parametrize("bad_sample", [True, 32_768, -32_769, object()])
def test_play_samples_rejects_invalid_pcm_values(bad_sample: object) -> None:
    with pytest.raises((TypeError, ValueError)):
        play_samples([bad_sample], sample_rate_hz=44_100)


@pytest.mark.parametrize("bad_rate", [True, 0, -1, 44_100.5, float("nan"), object()])
def test_play_samples_rejects_invalid_sample_rates(bad_rate: object) -> None:
    with pytest.raises((TypeError, ValueError)):
        play_samples([], sample_rate_hz=bad_rate)


def test_play_samples_rejects_non_mono_channel_counts() -> None:
    with pytest.raises(ValueError, match="mono"):
        play_samples([], sample_rate_hz=44_100, channel_count=2)


def test_play_pcm_buffer_requires_pcm_buffer_shape() -> None:
    with pytest.raises(TypeError, match="PCMBuffer"):
        play_pcm_buffer(object())


def test_empty_native_playback_returns_report_without_opening_device() -> None:
    report = play_samples([], sample_rate_hz=44_100)

    assert report == PlaybackReport(
        frames_played=0,
        sample_rate_hz=44_100,
        channel_count=1,
        duration_seconds=0.0,
        backend_name="coreaudio",
    )


def test_private_native_boundary_rejects_bool_samples() -> None:
    with pytest.raises(ValueError, match="signed 16-bit PCM"):
        audio_device_sink_module._play_samples([True], 44_100, 1)


@pytest.mark.skipif(sys.platform == "darwin", reason="macOS has the Core Audio backend")
def test_non_macos_non_empty_playback_raises_readable_error() -> None:
    with pytest.raises(AudioDeviceError, match="Core Audio"):
        play_samples([0], sample_rate_hz=44_100)


def test_module_exports_remain_stable() -> None:
    assert audio_device_sink_module.__all__ == [
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
