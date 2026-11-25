"""Property-based testing for wav_sink using SplitMix64 PRNG (MUS01, VER15)."""

from __future__ import annotations

from pathlib import Path

from pcm_audio import MAX_WAV_BYTES, PCM16_MAX, PCM16_MIN, PCMBuffer, PCMFormat

from wav_sink import (
    calc_total_wav_bytes,
    check_wav_size_bounds,
    from_wav_bytes,
    read_wav,
    to_wav_bytes,
    write_wav,
)

try:
    from coding_adventures_test_bootstrap.prng import SplitMix64
except ImportError:

    class SplitMix64:  # type: ignore[no-redef]
        """Minimal fallback SplitMix64 PRNG for isolated test environments."""

        def __init__(self, seed: int = 0) -> None:
            self._state = seed & 0xFFFF_FFFF_FFFF_FFFF

        def next_u64(self) -> int:
            self._state = (self._state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
            z = self._state
            z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
            z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
            return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

        def next_float(self) -> float:
            return (self.next_u64() >> 11) * (1.0 / (1 << 53))

        def bounded(self, n: int) -> int:
            threshold = (-n) % n
            while True:
                x = self.next_u64()
                if x >= threshold:
                    return x % n


def test_pbt_wav_size_algebra_and_bounds() -> None:
    rng = SplitMix64(0x5555_AAAA_1111_9999)
    for _ in range(200):
        payload = rng.bounded(MAX_WAV_BYTES * 2)
        total = calc_total_wav_bytes(payload)
        assert total == 44 + payload
        assert check_wav_size_bounds(total) == (44 <= total <= MAX_WAV_BYTES)


def test_pbt_wav_bytes_and_disk_roundtrip_under_base_dir(tmp_path: Path) -> None:
    rng = SplitMix64(0xCAFE_DEAD_BEEF_4321)
    rates = (8000.0, 11025.0, 16000.0, 22050.0, 44100.0, 48000.0, 96000.0)
    span = PCM16_MAX - PCM16_MIN + 1

    for idx in range(25):
        sr = rates[rng.bounded(len(rates))]
        count = rng.bounded(300)
        samples = tuple(PCM16_MIN + rng.bounded(span) for _ in range(count))
        pcm = PCMBuffer(samples, PCMFormat(sample_rate_hz=sr))

        wav_bytes = to_wav_bytes(pcm)
        assert len(wav_bytes) == calc_total_wav_bytes(count * 2)
        assert check_wav_size_bounds(len(wav_bytes))
        assert from_wav_bytes(wav_bytes).samples == samples

        target = tmp_path / f"pbt_{idx}.wav"
        written = write_wav(target, pcm, base_dir=tmp_path)
        assert written == target
        recovered = read_wav(target, base_dir=tmp_path)
        assert recovered.samples == samples
        assert recovered.pcm_format.sample_rate_hz == sr
