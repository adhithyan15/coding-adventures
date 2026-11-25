"""Property-based testing for audio_device_sink using SplitMix64 PRNG (MUS02, VER15)."""

from __future__ import annotations

import math

import pytest
from pcm_audio import PCMBuffer, PCMFormat

import audio_device_sink as audio_device_sink_module
from audio_device_sink import (
    MAX_BLOCKING_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    PCM16_MAX,
    PCM16_MIN,
    PlaybackReport,
    play_pcm_buffer,
    play_samples,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:

    class SplitMix64:  # type: ignore[no-redef]
        def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
            self.state: int = seed & 0xFFFFFFFFFFFFFFFF

        def next_u64(self) -> int:
            self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
            z = self.state
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
            return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF


def test_pbt_play_samples_and_buffer_invariants(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    prng = SplitMix64(0x0A0D_10D3_71CE_51A4)

    def fake_play(samples: list[int], sample_rate_hz: int, channel_count: int):
        return (
            len(samples),
            sample_rate_hz,
            channel_count,
            len(samples) / sample_rate_hz,
            "fake-pbt",
        )

    monkeypatch.setattr(audio_device_sink_module, "_play_samples", fake_play)

    for _ in range(100):
        rate = 1 + (prng.next_u64() % MAX_SAMPLE_RATE_HZ)
        max_len = min(rate * 600, 512)
        count = prng.next_u64() % (max_len + 1)
        samples = [
            PCM16_MIN + (prng.next_u64() % (PCM16_MAX - PCM16_MIN + 1))
            for _ in range(count)
        ]

        r1 = play_samples(samples, sample_rate_hz=rate)
        r2 = play_samples(tuple(samples), sample_rate_hz=float(rate))
        assert r1 == r2
        assert isinstance(r1, PlaybackReport)
        assert r1.frames_played == count
        assert r1.sample_rate_hz == rate
        assert r1.channel_count == 1
        assert math.isfinite(r1.duration_seconds)
        assert 0.0 <= r1.duration_seconds <= MAX_BLOCKING_DURATION_SECONDS

        if rate <= 192_000:
            buf = PCMBuffer(
                samples=tuple(samples),
                pcm_format=PCMFormat(sample_rate_hz=float(rate)),
            )
            rbuf = play_pcm_buffer(buf)
            assert rbuf == r1


def test_pbt_hostile_rates_and_samples_always_fail_cleanly() -> None:
    prng = SplitMix64(0x0BAD_F00D_1234_5678)

    for _ in range(100):
        bad_rate = MAX_SAMPLE_RATE_HZ + 1 + (prng.next_u64() % 1_000_000)
        with pytest.raises(ValueError):
            play_samples([], sample_rate_hz=bad_rate)

        bad_sample = (
            PCM16_MAX + 1 + (prng.next_u64() % 100_000)
            if (prng.next_u64() & 1) == 0
            else PCM16_MIN - 1 - (prng.next_u64() % 100_000)
        )
        with pytest.raises(ValueError):
            play_samples([bad_sample], sample_rate_hz=44_100)
