"""Property-based testing for pcm_audio using SplitMix64 PRNG (MUS01, VER15)."""

from __future__ import annotations

import struct

from oscillator import SampleBuffer

from pcm_audio import (
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
    _safe_round_sample,
    encode_sample_buffer,
    from_wav_bytes,
    validate_wav_header,
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


def test_pbt_safe_round_sample_monotonicity_and_bounds() -> None:
    rng = SplitMix64(0x1234_5678_9ABC_DEF0)
    for _ in range(200):
        x = (rng.next_float() * 4.0) - 2.0
        y = (rng.next_float() * 4.0) - 2.0
        qx, clip_x = _safe_round_sample(x)
        qy, clip_y = _safe_round_sample(y)

        assert PCM16_MIN <= qx <= PCM16_MAX
        assert PCM16_MIN <= qy <= PCM16_MAX
        assert clip_x == (x < -1.0 or x > 1.0)
        assert clip_y == (y < -1.0 or y > 1.0)
        if x <= y:
            assert qx <= qy


def test_pbt_encode_and_wav_bytes_roundtrip() -> None:
    rng = SplitMix64(0xCAFE_0102_0304_0506)
    rates = (8000.0, 16000.0, 22050.0, 44100.0, 48000.0)
    for _ in range(40):
        rate = rates[rng.bounded(len(rates))]
        n = rng.bounded(200)
        raw = tuple((rng.next_float() * 2.4) - 1.2 for _ in range(n))
        sbuf = SampleBuffer(samples=raw, sample_rate_hz=rate)
        pcm = encode_sample_buffer(sbuf)

        assert pcm.sample_count() == n
        le_bytes = pcm.to_little_endian_bytes()
        assert len(le_bytes) == n * 2

        fmt = struct.pack(
            "<4sIHHIIHH",
            b"fmt ",
            16,
            1,
            1,
            int(rate),
            int(rate) * 2,
            2,
            16,
        )
        data = struct.pack("<4sI", b"data", len(le_bytes)) + le_bytes
        wav_bytes = (
            struct.pack("<4sI4s", b"RIFF", 4 + len(fmt) + len(data), b"WAVE")
            + fmt
            + data
        )
        recovered = from_wav_bytes(wav_bytes)
        assert recovered.samples == pcm.samples
        assert recovered.pcm_format.sample_rate_hz == rate


def test_pbt_corrupt_wav_bytes_fuzz_fails_closed() -> None:
    rng = SplitMix64(0xDEAD_BEEF_1357_2468)
    for _ in range(300):
        length = rng.bounded(128)
        payload = bytes(rng.bounded(256) for _ in range(length))
        try:
            channels, rate, depth, data_off, data_len = validate_wav_header(payload)
            assert 1 <= channels <= 8
            assert 1 <= rate <= 192_000
            assert depth in (16, 32)
            assert data_off >= 20
            assert data_off + data_len <= len(payload)
        except ValueError:
            pass
    # Also verify empty PCMBuffer roundtrips timing
    empty_buf = PCMBuffer((), PCMFormat(sample_rate_hz=44_100.0))
    assert empty_buf.duration_seconds() == 0.0
