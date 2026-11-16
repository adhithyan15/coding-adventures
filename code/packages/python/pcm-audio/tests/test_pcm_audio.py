"""Tests for the reusable PCM encoding stage."""

from __future__ import annotations

import math
import struct
from collections.abc import Iterator

import pytest
from oscillator import SampleBuffer

from pcm_audio import (
    DEFAULT_BIT_DEPTH,
    DEFAULT_CHANNEL_COUNT,
    DEFAULT_FULL_SCALE_VOLTAGE,
    DEFAULT_SAMPLE_RATE_HZ,
    MAX_CHANNELS,
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MAX_WAV_BYTES,
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
    __version__,
    _safe_round_sample,
    encode_sample_buffer,
    float_to_pcm16,
    from_wav_bytes,
    safe_round_sample,
    samples_to_pcm_buffer,
    validate_wav_header,
)


def assert_close(got: float, want: float, tolerance: float = 1e-9) -> None:
    """Keep timing assertions readable."""
    assert abs(got - want) <= tolerance


# =============================================================================
# § 1. Pure Python SplitMix64 Deterministic PRNG for PBT / Fuzzing
# =============================================================================


class SplitMix64:
    """Deterministic 64-bit PRNG (Steele et al., 2014) for reproducible PBT."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
        return z ^ (z >> 31)

    def next_float(self) -> float:
        """Return uniform float in [0.0, 1.0)."""
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, low: float, high: float) -> float:
        """Return uniform float in [low, high)."""
        return low + (high - low) * self.next_float()


def test_version_and_defaults_are_visible() -> None:
    assert __version__ == "0.1.0"
    assert DEFAULT_SAMPLE_RATE_HZ == 44_100.0
    assert DEFAULT_CHANNEL_COUNT == 1
    assert DEFAULT_BIT_DEPTH == 16
    assert DEFAULT_FULL_SCALE_VOLTAGE == 1.0
    assert PCM16_MIN == -32_768
    assert PCM16_MAX == 32_767
    assert MAX_SAMPLE_RATE_HZ == 192_000.0
    assert MAX_DURATION_SECONDS == 60.0
    assert MAX_SAMPLES == 16_777_216
    assert MAX_CHANNELS == 8
    assert MAX_WAV_BYTES == 67_108_864


def test_pcm_format_validates_v1_shape() -> None:
    pcm_format = PCMFormat(sample_rate_hz=8.0, full_scale_voltage=2.0)

    assert pcm_format.minimum_integer == -32_768
    assert pcm_format.maximum_integer == 32_767
    assert pcm_format.sample_width_bytes == 2
    assert pcm_format.integer_sample_rate() == 8


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"sample_rate_hz": 0.0}, "sample_rate_hz"),
        ({"sample_rate_hz": math.inf}, "sample_rate_hz"),
        ({"sample_rate_hz": MAX_SAMPLE_RATE_HZ + 1.0}, "MAX_SAMPLE_RATE_HZ"),
        ({"channel_count": 2}, "mono"),
        ({"channel_count": 9}, "MAX_CHANNELS"),
        ({"channel_count": True}, "channel_count"),
        ({"bit_depth": 24}, "16-bit"),
        ({"bit_depth": 0}, "bit_depth"),
        ({"full_scale_voltage": 0.0}, "full_scale_voltage"),
    ],
)
def test_pcm_format_rejects_invalid_shapes(
    kwargs: dict[str, object],
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        PCMFormat(**kwargs)


def test_integer_sample_rate_rejects_fractional_rates() -> None:
    with pytest.raises(ValueError, match="integer-valued"):
        PCMFormat(sample_rate_hz=44_100.5).integer_sample_rate()


def test_float_to_pcm16_clips_and_reports_clipping() -> None:
    assert float_to_pcm16(0.0) == (0, False)
    assert float_to_pcm16(1.0) == (32767, False)
    assert float_to_pcm16(-1.0) == (-32768, False)
    assert float_to_pcm16(2.0) == (32767, True)
    assert float_to_pcm16(-2.0) == (-32768, True)
    assert _safe_round_sample(0.5) == safe_round_sample(0.5)


@pytest.mark.parametrize(
    "sample", [math.nan, math.inf, -math.inf, True, False, object()]
)
def test_float_to_pcm16_rejects_non_finite_or_non_real_samples(
    sample: object,
) -> None:
    with pytest.raises(ValueError, match="sample"):
        float_to_pcm16(sample)  # type: ignore[arg-type]


def test_encode_sample_buffer_tracks_clipping_and_timing() -> None:
    floating = SampleBuffer(
        samples=(0.0, 1.0, -1.0, 2.0),
        sample_rate_hz=4.0,
        start_time_seconds=10.0,
    )
    pcm = encode_sample_buffer(floating)

    assert pcm.samples == (0, 32767, -32768, 32767)
    assert pcm.clipped_sample_count == 1
    assert pcm.sample_count() == 4
    assert_close(pcm.sample_period_seconds(), 0.25)
    assert_close(pcm.duration_seconds(), 1.0)
    assert_close(pcm.time_at(2), 10.5)


def test_samples_to_pcm_buffer_encodes_raw_float_sequences() -> None:
    pcm = samples_to_pcm_buffer((0.0, 0.5, -0.5), sample_rate_hz=3.0)

    assert pcm.samples == (0, 16384, -16384)
    assert_close(pcm.duration_seconds(), 1.0)


def test_pcm_buffer_packs_little_endian_bytes() -> None:
    pcm = PCMBuffer((0, 32767, -32768), PCMFormat(sample_rate_hz=3.0))

    assert pcm.to_little_endian_bytes() == b"\x00\x00\xff\x7f\x00\x80"


@pytest.mark.parametrize(
    ("samples", "message"),
    [
        ((True,), "PCM integer"),
        ((32768,), "signed 16-bit"),
        ((-32769,), "signed 16-bit"),
    ],
)
def test_pcm_buffer_rejects_invalid_integer_samples(
    samples: tuple[object, ...],
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        PCMBuffer(samples, PCMFormat())


def test_pcm_buffer_rejects_invalid_metadata() -> None:
    with pytest.raises(ValueError, match="pcm_format"):
        PCMBuffer((0,), object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="start_time_seconds"):
        PCMBuffer((0,), PCMFormat(), start_time_seconds=math.nan)
    with pytest.raises(ValueError, match="clipped_sample_count"):
        PCMBuffer((0,), PCMFormat(), clipped_sample_count=-1)
    with pytest.raises(ValueError, match="clipped_sample_count"):
        PCMBuffer((0,), PCMFormat(), clipped_sample_count=True)  # type: ignore[arg-type]


def test_pcm_buffer_rejects_invalid_indexes() -> None:
    pcm = PCMBuffer((0,), PCMFormat())
    with pytest.raises(ValueError, match="index"):
        pcm.time_at(1)
    with pytest.raises(ValueError, match="index"):
        pcm.time_at(True)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="index"):
        pcm.time_at(-1)


def test_encode_sample_buffer_rejects_non_sample_buffer() -> None:
    with pytest.raises(ValueError, match="sample_buffer"):
        encode_sample_buffer(object())  # type: ignore[arg-type]


def test_pcm_buffer_rejects_oversized_allocation() -> None:
    class FakeOversizedSeq:
        def __len__(self) -> int:
            return MAX_SAMPLES + 1

        def __iter__(self) -> Iterator[int]:
            return iter(())

    with pytest.raises(ValueError, match="MAX_SAMPLES"):
        PCMBuffer(samples=FakeOversizedSeq(), pcm_format=PCMFormat())  # type: ignore[arg-type]


def test_pcm_buffer_to_little_endian_bytes_rejects_oversized(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import pcm_audio.pcm_audio as pcm_mod

    monkeypatch.setattr(pcm_mod, "MAX_WAV_BYTES", 4)
    pcm = PCMBuffer((0, 1, 2), PCMFormat())
    with pytest.raises(ValueError, match="MAX_WAV_BYTES"):
        pcm.to_little_endian_bytes()


def test_encode_sample_buffer_rejects_oversized(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import pcm_audio.pcm_audio as pcm_mod

    monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 2)
    buf = SampleBuffer(samples=(0.0, 0.1, 0.2), sample_rate_hz=100.0)
    with pytest.raises(ValueError, match="MAX_SAMPLES"):
        encode_sample_buffer(buf)


def test_samples_to_pcm_buffer_rejects_oversized(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import pcm_audio.pcm_audio as pcm_mod

    monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 2)
    with pytest.raises(ValueError, match="MAX_SAMPLES"):
        samples_to_pcm_buffer((0.0, 0.1, 0.2), sample_rate_hz=100.0)


# =============================================================================
# § 2. WAV Validation and Deserialization Tests (CWE-400 / CWE-20)
# =============================================================================


def _build_test_wav_bytes(
    channels: int = 1,
    sample_rate: int = 44100,
    bits_per_sample: int = 16,
    audio_format: int = 1,
    pcm_data: bytes = b"\x00\x00\x00\x00",
) -> bytes:
    block_align = channels * (bits_per_sample // 8)
    byte_rate = sample_rate * block_align
    fmt_chunk = struct.pack(
        "<4sIHHIIHH",
        b"fmt ",
        16,
        audio_format,
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bits_per_sample,
    )
    data_chunk = struct.pack("<4sI", b"data", len(pcm_data)) + pcm_data
    riff_len = 4 + len(fmt_chunk) + len(data_chunk)
    header = struct.pack("<4sI4s", b"RIFF", riff_len, b"WAVE")
    return header + fmt_chunk + data_chunk


def test_validate_wav_header_and_from_wav_bytes_roundtrip() -> None:
    wav_bytes = _build_test_wav_bytes(
        channels=1,
        sample_rate=44100,
        bits_per_sample=16,
        pcm_data=b"\x00\x00\xff\x7f",
    )
    channels, rate, depth, offset, length = validate_wav_header(wav_bytes)
    assert channels == 1
    assert rate == 44100
    assert depth == 16
    assert length == 4

    pcm = from_wav_bytes(wav_bytes)
    assert pcm.sample_count() == 2
    assert pcm.samples == (0, 32767)


def test_from_wav_bytes_float_audio_format() -> None:
    fbytes = struct.pack("<ff", 0.0, 2.0)  # 2.0 triggers clipped branch in float
    wav_bytes = _build_test_wav_bytes(
        channels=1,
        sample_rate=48000,
        bits_per_sample=32,
        audio_format=3,
        pcm_data=fbytes,
    )
    pcm = from_wav_bytes(wav_bytes)
    assert pcm.sample_count() == 2
    assert pcm.samples == (0, 32767)
    assert pcm.clipped_sample_count == 1


def test_validate_wav_header_rejects_unaligned_data_chunk() -> None:
    wav_bytes = _build_test_wav_bytes(
        channels=1,
        sample_rate=44100,
        bits_per_sample=16,
        pcm_data=b"\x00\x00\x00",
    )
    with pytest.raises(ValueError, match="multiple of block align"):
        validate_wav_header(wav_bytes)


def test_from_wav_bytes_rejects_multi_channel() -> None:
    wav_bytes = _build_test_wav_bytes(
        channels=2,
        sample_rate=44100,
        bits_per_sample=16,
        pcm_data=b"\x00\x00\x00\x00",
    )
    with pytest.raises(ValueError, match="only mono WAV files are supported"):
        from_wav_bytes(wav_bytes)


@pytest.mark.parametrize(
    ("corrupt_bytes", "msg"),
    [
        (b"RIFF1234NOTW".ljust(44, b"\x00"), "WAV"),
        (b"NOTR1234WAVE".ljust(44, b"\x00"), "RIFF"),
        (b"RIFF\x00\x00\x00\x00WAVE".ljust(44, b"\x00"), "RIFF payload size too small"),
        (b"short", "smaller than minimum header size"),
    ],
)
def test_validate_wav_header_rejects_corrupted_magic(
    corrupt_bytes: bytes,
    msg: str,
) -> None:
    with pytest.raises(ValueError, match=msg):
        validate_wav_header(corrupt_bytes)


def test_validate_wav_header_rejects_non_bytes() -> None:
    with pytest.raises(ValueError, match="wav_bytes must be bytes"):
        validate_wav_header("not bytes")  # type: ignore[arg-type]


def test_validate_wav_header_rejects_oversized(monkeypatch: pytest.MonkeyPatch) -> None:
    import pcm_audio.pcm_audio as pcm_mod

    monkeypatch.setattr(pcm_mod, "MAX_WAV_BYTES", 10)
    with pytest.raises(ValueError, match="MAX_WAV_BYTES"):
        validate_wav_header(b"0" * 20)


def test_validate_wav_header_rejects_chunk_exceeding_buffer() -> None:
    bad_chunk = struct.pack("<4sI4s4sI", b"RIFF", 100, b"WAVE", b"fmt ", 1000)
    with pytest.raises(ValueError, match="exceeds buffer"):
        validate_wav_header(bad_chunk.ljust(50, b"\x00"))


def test_validate_wav_header_rejects_unsupported_audio_format() -> None:
    wav_bytes = _build_test_wav_bytes(audio_format=7)
    with pytest.raises(ValueError, match="unsupported WAV audio format"):
        validate_wav_header(wav_bytes)


def test_validate_wav_header_rejects_bad_channel_count() -> None:
    wav_bytes = _build_test_wav_bytes(channels=0)
    with pytest.raises(ValueError, match="unsupported channel count"):
        validate_wav_header(wav_bytes)


def test_validate_wav_header_rejects_bad_sample_rate() -> None:
    wav_bytes = _build_test_wav_bytes(sample_rate=0)
    with pytest.raises(ValueError, match="sample rate 0"):
        validate_wav_header(wav_bytes)


def test_validate_wav_header_rejects_bad_bit_depth() -> None:
    wav_bytes = _build_test_wav_bytes(bits_per_sample=24)
    with pytest.raises(ValueError, match="unsupported bit depth"):
        validate_wav_header(wav_bytes)


def test_validate_wav_header_rejects_bad_float_bit_depth() -> None:
    # Float format with 16-bit instead of 32-bit
    block_align = 2
    byte_rate = 44100 * block_align
    fmt = struct.pack(
        "<4sIHHIIHH",
        b"fmt ",
        16,
        3,  # float
        1,  # mono
        44100,
        byte_rate,
        block_align,
        16,  # invalid for float!
    )
    header = struct.pack("<4sI4s", b"RIFF", 4 + len(fmt) + 8, b"WAVE")
    data = struct.pack("<4sI", b"data", 0)
    with pytest.raises(ValueError, match="IEEE float"):
        validate_wav_header(header + fmt + data)


def test_validate_wav_header_rejects_small_fmt_chunk() -> None:
    # fmt chunk declaring size 10 (< 16), padded to exceed 44-byte min
    fmt = struct.pack("<4sI", b"fmt ", 10) + b"\x00" * 10
    extra = struct.pack("<4sI", b"JUNK", 8) + b"\x00" * 8
    header = struct.pack("<4sI4s", b"RIFF", 4 + len(fmt) + len(extra) + 8, b"WAVE")
    data = struct.pack("<4sI", b"data", 0)
    with pytest.raises(ValueError, match="fmt chunk size too small"):
        validate_wav_header(header + fmt + extra + data)


def test_finite_float_rejects_non_finite_start_time() -> None:
    with pytest.raises(ValueError, match="start_time_seconds must be finite"):
        samples_to_pcm_buffer(
            (0.0,), sample_rate_hz=100.0, start_time_seconds=float("nan")
        )
    with pytest.raises(ValueError, match="start_time_seconds must be a finite real"):
        samples_to_pcm_buffer(
            (0.0,),
            sample_rate_hz=100.0,
            start_time_seconds="not_a_number",  # type: ignore[arg-type]
        )


def test_validate_wav_header_rejects_inconsistent_block_align() -> None:
    wav = bytearray(_build_test_wav_bytes())
    wav[32] = 99
    with pytest.raises(ValueError, match="inconsistent block align"):
        validate_wav_header(bytes(wav))


def test_validate_wav_header_rejects_inconsistent_byte_rate() -> None:
    wav = bytearray(_build_test_wav_bytes())
    wav[28] = 99
    with pytest.raises(ValueError, match="inconsistent byte rate"):
        validate_wav_header(bytes(wav))


def test_validate_wav_header_rejects_missing_chunks() -> None:
    # 44 bytes total, but data chunk appears before fmt
    header = struct.pack("<4sI4s", b"RIFF", 36, b"WAVE")
    data_chunk = struct.pack("<4sI", b"data", 24) + b"\x00" * 24
    with pytest.raises(ValueError, match="data chunk before fmt chunk"):
        validate_wav_header(header + data_chunk)


def test_validate_wav_header_rejects_missing_fmt_and_data() -> None:
    # 44 bytes, valid other chunk but no fmt and no data
    header = struct.pack("<4sI4s", b"RIFF", 36, b"WAVE")
    other_chunk = struct.pack("<4sI", b"JUNK", 24) + b"\x00" * 24
    with pytest.raises(ValueError, match="missing fmt chunk"):
        validate_wav_header(header + other_chunk)

    # Valid fmt chunk, but no data chunk
    fmt = struct.pack("<4sIHHIIHH", b"fmt ", 16, 1, 1, 44100, 88200, 2, 16)
    other = struct.pack("<4sI", b"JUNK", 8) + b"\x00" * 8
    hdr = struct.pack("<4sI4s", b"RIFF", 4 + len(fmt) + len(other), b"WAVE")
    with pytest.raises(ValueError, match="missing data chunk"):
        validate_wav_header(hdr + fmt + other)


def test_from_wav_bytes_rejects_sample_count_too_large(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import pcm_audio.pcm_audio as pcm_mod

    monkeypatch.setattr(pcm_mod, "MAX_SAMPLES", 1)
    wav_bytes = _build_test_wav_bytes(pcm_data=b"\x00\x00\x00\x00")  # 2 samples
    with pytest.raises(ValueError, match="MAX_SAMPLES"):
        from_wav_bytes(wav_bytes)


# =============================================================================
# § 3. Property-Based Testing (PBT) using SplitMix64 PRNG
# =============================================================================


def test_splitmix64_pbt_safe_round_sample_invariants() -> None:
    """PBT: Quantized PCM16 samples are strictly within [PCM16_MIN, PCM16_MAX]."""
    prng = SplitMix64(seed=0xCAFE_BABE_0001)

    for _ in range(100):
        val = prng.uniform(-3.0, 3.0)
        pcm_val, clipped = _safe_round_sample(val)

        assert PCM16_MIN <= pcm_val <= PCM16_MAX
        if -1.0 <= val <= 1.0:
            assert not clipped
        else:
            assert clipped


def test_splitmix64_pbt_pcm_buffer_little_endian_roundtrip() -> None:
    """PBT: Unpacking little-endian bytes exactly recovers original PCM integers."""
    prng = SplitMix64(seed=0xCAFE_BABE_0002)

    for _ in range(25):
        length = (prng.next_u64() % 50) + 1
        samples: list[int] = []
        for _ in range(length):
            s = (prng.next_u64() % 65536) - 32768
            samples.append(s)

        buf = PCMBuffer(tuple(samples), PCMFormat())
        raw_bytes = buf.to_little_endian_bytes()
        assert len(raw_bytes) == length * 2

        recovered = [
            struct.unpack_from("<h", raw_bytes, i * 2)[0] for i in range(length)
        ]
        assert recovered == samples
