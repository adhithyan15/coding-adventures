"""Deterministic PBT and adversarial test suite for wav_sink."""

from __future__ import annotations

import stat
import wave
from io import BytesIO
from pathlib import Path
from unittest.mock import patch

import pytest
from pcm_audio import (
    MAX_CHANNELS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MAX_WAV_BYTES,
    PCMBuffer,
    PCMFormat,
)

from wav_sink import (
    __version__,
    calc_total_wav_bytes,
    check_wav_size_bounds,
    read_wav,
    to_wav_bytes,
    validate_wav_path,
    write_wav,
)


class SplitMix64:
    """Deterministic SplitMix64 PRNG for property-based tests."""

    def __init__(self, seed: int) -> None:
        self._state = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self._state = (self._state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self._state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

    def next_int(self, min_val: int, max_val: int) -> int:
        span = max_val - min_val + 1
        return min_val + (self.next_u64() % span)

    def next_float(self, min_val: float, max_val: float) -> float:
        raw = self.next_u64() / float(0xFFFFFFFFFFFFFFFF)
        return min_val + raw * (max_val - min_val)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_calc_total_wav_bytes() -> None:
    assert calc_total_wav_bytes(0) == 44
    assert calc_total_wav_bytes(100) == 144
    assert calc_total_wav_bytes(1000) == 1044

    with pytest.raises(TypeError, match="payload must be an int"):
        calc_total_wav_bytes(True)  # type: ignore[arg-type]

    with pytest.raises(TypeError, match="payload must be an int"):
        calc_total_wav_bytes("100")  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="payload must be non-negative"):
        calc_total_wav_bytes(-1)


def test_check_wav_size_bounds() -> None:
    assert not check_wav_size_bounds(True)  # type: ignore[arg-type]
    assert not check_wav_size_bounds("44")  # type: ignore[arg-type]
    assert not check_wav_size_bounds(43)
    assert check_wav_size_bounds(44)
    assert check_wav_size_bounds(1000)
    assert check_wav_size_bounds(MAX_WAV_BYTES)
    assert not check_wav_size_bounds(MAX_WAV_BYTES + 1)
    assert not check_wav_size_bounds(70_000_000)


def test_validate_wav_path_type_errors() -> None:
    with pytest.raises(TypeError, match="path must be a str or PathLike"):
        validate_wav_path(True)  # type: ignore[arg-type]

    with pytest.raises(TypeError, match="path must be a str or PathLike"):
        validate_wav_path(123)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="null bytes"):
        validate_wav_path("audio\0.wav")

    with pytest.raises(ValueError, match="must not be empty"):
        validate_wav_path("   ")


def test_validate_wav_path_traversal() -> None:
    with pytest.raises(ValueError, match="Path traversal"):
        validate_wav_path("sub/../file.wav")

    with pytest.raises(ValueError, match="Path traversal"):
        validate_wav_path("../file.wav")

    with pytest.raises(ValueError, match="Path traversal"):
        validate_wav_path("file.wav", base_dir="foo/../bar")


def test_validate_wav_path_extension() -> None:
    with pytest.raises(ValueError, match="Path must end in .wav or .wave"):
        validate_wav_path("file.mp3")

    with pytest.raises(ValueError, match="Path must end in .wav or .wave"):
        validate_wav_path("file")

    p1 = validate_wav_path("sound.WAV")
    assert p1.suffix.lower() == ".wav"

    p2 = validate_wav_path("sound.wave")
    assert p2.suffix.lower() == ".wave"


def test_validate_wav_path_base_dir(tmp_path: Path) -> None:
    base = tmp_path / "audio_root"
    base.mkdir()

    with pytest.raises(TypeError, match="base_dir must be a str or PathLike"):
        validate_wav_path("sound.wav", base_dir=True)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="base_dir must not contain null bytes"):
        validate_wav_path("sound.wav", base_dir="dir\0")

    with pytest.raises(ValueError, match="does not exist"):
        validate_wav_path("sound.wav", base_dir=tmp_path / "nonexistent")

    non_dir = tmp_path / "dummy.txt"
    non_dir.write_text("hello")
    with pytest.raises(ValueError, match="is not a directory"):
        validate_wav_path("sound.wav", base_dir=non_dir)

    # Path escaping base_dir
    with pytest.raises(ValueError, match="escapes base directory"):
        validate_wav_path(tmp_path / "outside.wav", base_dir=base)

    # Valid path in base_dir
    inside = base / "nested" / "sound.wav"
    inside.parent.mkdir()
    validated = validate_wav_path(inside, base_dir=base)
    assert validated == inside


def test_validate_wav_path_symlinks(tmp_path: Path) -> None:
    f = tmp_path / "test.wav"
    f.touch()

    with (
        patch.object(Path, "is_symlink", return_value=True),
        pytest.raises(ValueError, match="is a symlink"),
    ):
        validate_wav_path(f)

    # Ancestor is symlink
    parent_dir = tmp_path / "sub"
    parent_dir.mkdir()
    target_in_sub = parent_dir / "test.wav"
    target_in_sub.touch()

    def mock_is_symlink(self: Path) -> bool:
        return self == parent_dir

    with (
        patch.object(Path, "is_symlink", mock_is_symlink),
        pytest.raises(ValueError, match="ancestor .* is a symlink"),
    ):
        validate_wav_path(target_in_sub)

    # Base dir itself is symlink
    base_dir = tmp_path / "base"
    base_dir.mkdir()

    def mock_base_symlink(self: Path) -> bool:
        return self == base_dir

    with (
        patch.object(Path, "is_symlink", mock_base_symlink),
        pytest.raises(ValueError, match="is a symlink"),
    ):
        validate_wav_path(base_dir / "test.wav", base_dir=base_dir)


def test_validate_wav_path_existence(tmp_path: Path) -> None:
    non_existent = tmp_path / "nonexistent.wav"

    with pytest.raises(FileNotFoundError, match="does not exist"):
        validate_wav_path(non_existent, must_exist=True)

    folder_with_wav_name = tmp_path / "folder.wav"
    folder_with_wav_name.mkdir()

    with pytest.raises(ValueError, match="not a regular file"):
        validate_wav_path(folder_with_wav_name, must_exist=True)

    with pytest.raises(ValueError, match="exists and is not a regular file"):
        validate_wav_path(folder_with_wav_name, must_exist=False)

    # Parent exists and is not a directory
    file_as_parent = tmp_path / "not_a_dir"
    file_as_parent.write_text("content")
    bad_parent_path = file_as_parent / "test.wav"
    with pytest.raises(ValueError, match="Parent .* is not a directory"):
        validate_wav_path(bad_parent_path, must_exist=False)


def test_to_wav_bytes_basic() -> None:
    pcm = PCMBuffer((0, 32767, -32768), PCMFormat(sample_rate_hz=8.0))
    wav_bytes = to_wav_bytes(pcm)

    assert wav_bytes.startswith(b"RIFF")
    assert wav_bytes[8:12] == b"WAVE"
    with wave.open(BytesIO(wav_bytes), "rb") as wav_file:
        assert wav_file.getnchannels() == 1
        assert wav_file.getsampwidth() == 2
        assert wav_file.getframerate() == 8
        assert wav_file.getnframes() == 3
        assert wav_file.readframes(3) == b"\x00\x00\xff\x7f\x00\x80"


def test_to_wav_bytes_input_validation() -> None:
    with pytest.raises(ValueError, match="pcm_buffer must be a PCMBuffer"):
        to_wav_bytes(object())  # type: ignore[arg-type]

    # Non-integer sample rate
    pcm_non_int = PCMBuffer((0,), PCMFormat(sample_rate_hz=44_100.5))
    with pytest.raises(ValueError, match="integer-valued"):
        to_wav_bytes(pcm_non_int)

    # Sample rate out of range / bool / nan / inf
    bad_fmt1 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_fmt1, "sample_rate_hz", True)
    with pytest.raises(TypeError, match="sample_rate_hz must be numeric"):
        to_wav_bytes(PCMBuffer((0,), bad_fmt1))

    bad_fmt2 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_fmt2, "sample_rate_hz", float("nan"))
    with pytest.raises(ValueError, match="NaN or Inf"):
        to_wav_bytes(PCMBuffer((0,), bad_fmt2))

    bad_fmt3 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_fmt3, "sample_rate_hz", float("inf"))
    with pytest.raises(ValueError, match="NaN or Inf"):
        to_wav_bytes(PCMBuffer((0,), bad_fmt3))

    bad_fmt4 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_fmt4, "sample_rate_hz", 0.0)
    with pytest.raises(ValueError, match="sample_rate_hz must be between 1"):
        to_wav_bytes(PCMBuffer((0,), bad_fmt4))

    bad_fmt5 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_fmt5, "sample_rate_hz", MAX_SAMPLE_RATE_HZ + 1000)
    with pytest.raises(ValueError, match="sample_rate_hz must be between 1"):
        to_wav_bytes(PCMBuffer((0,), bad_fmt5))

    # Bad channel count
    bad_ch1 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_ch1, "channel_count", 0)
    with pytest.raises(ValueError, match="channel count 0 out of range"):
        to_wav_bytes(PCMBuffer((0,), bad_ch1))

    bad_ch2 = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_ch2, "channel_count", MAX_CHANNELS + 1)
    with pytest.raises(ValueError, match="channel count 9 out of range"):
        to_wav_bytes(PCMBuffer((0,), bad_ch2))

    # Bad sample width
    bad_width = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_width, "bit_depth", 24)
    with pytest.raises(ValueError, match="unsupported sample width"):
        to_wav_bytes(PCMBuffer((0,), bad_width))

    # Bad integer sample rate
    bad_irate = PCMFormat(sample_rate_hz=8.0)
    object.__setattr__(bad_irate, "integer_sample_rate", lambda: 0)
    with pytest.raises(ValueError, match="integer sample rate must be between 1"):
        to_wav_bytes(PCMBuffer((0,), bad_irate))


def test_to_wav_bytes_unbounded_limits() -> None:
    fmt = PCMFormat(sample_rate_hz=8.0)

    # Exceed MAX_SAMPLES
    buf = PCMBuffer((0,), fmt)
    object.__setattr__(buf, "sample_count", lambda: MAX_SAMPLES + 1)
    with pytest.raises(ValueError, match="sample count .* exceeds MAX_SAMPLES"):
        to_wav_bytes(buf)

    # Non-callable or invalid sample_count fallback
    buf_fallback = PCMBuffer((0,), fmt)
    object.__setattr__(buf_fallback, "sample_count", "not-a-number")
    wav_fallback = to_wav_bytes(buf_fallback)
    assert len(wav_fallback) == 46

    # Exceed MAX_DURATION_SECONDS
    long_samples = tuple(0 for _ in range(65))
    buf_long = PCMBuffer(long_samples, PCMFormat(sample_rate_hz=1.0))
    with pytest.raises(ValueError, match="exceeds MAX_DURATION_SECONDS"):
        to_wav_bytes(buf_long)

    # Exceed MAX_WAV_BYTES check
    buf_size = PCMBuffer((0,), fmt)
    with (
        patch("wav_sink.wav_sink.check_wav_size_bounds", return_value=False),
        pytest.raises(ValueError, match="exceeds MAX_WAV_BYTES"),
    ):
        to_wav_bytes(buf_size)

    # Serialized WAV size exceeds MAX_WAV_BYTES
    buf_ser = PCMBuffer((0,), fmt)

    class FakeWave:
        def __enter__(self) -> FakeWave:
            return self

        def __exit__(self, *args: object) -> None:
            pass

        def setnchannels(self, n: int) -> None:
            pass

        def setsampwidth(self, w: int) -> None:
            pass

        def setframerate(self, r: int) -> None:
            pass

        def writeframes(self, f: bytes) -> None:
            pass

    class FakeBytesIO(BytesIO):
        def getvalue(self) -> bytes:
            return b"x" * (MAX_WAV_BYTES + 10)

    with (
        patch("wav_sink.wav_sink.BytesIO", FakeBytesIO),
        patch("wav_sink.wav_sink.wave.open", return_value=FakeWave()),
        pytest.raises(ValueError, match="serialized WAV size .* exceeds MAX_WAV_BYTES"),
    ):
        to_wav_bytes(buf_ser)


def test_write_and_read_wav_roundtrip(tmp_path: Path) -> None:
    pcm = PCMBuffer((0, 100, -100, 32767, -32768), PCMFormat(sample_rate_hz=44_100.0))
    wav_path = tmp_path / "test_roundtrip.wav"

    written_path = write_wav(wav_path, pcm)
    assert written_path == wav_path
    assert wav_path.is_file()

    read_back = read_wav(wav_path)
    assert read_back.samples == pcm.samples
    assert read_back.pcm_format.sample_rate_hz == pcm.pcm_format.sample_rate_hz
    assert read_back.pcm_format.channel_count == pcm.pcm_format.channel_count


def test_write_wav_rejects_non_regular_file(tmp_path: Path) -> None:
    pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
    wav_path = tmp_path / "test_non_reg.wav"

    class MockNonRegStat:
        st_mode = stat.S_IFDIR

    with (
        patch("os.fstat", return_value=MockNonRegStat()),
        pytest.raises(ValueError, match="not a regular file"),
    ):
        write_wav(wav_path, pcm)


def test_read_wav_rejects_non_regular_file(tmp_path: Path) -> None:
    wav_path = tmp_path / "test_read_non_reg.wav"
    wav_path.touch()

    class MockNonRegStat:
        st_mode = stat.S_IFDIR

    with (
        patch("os.fstat", return_value=MockNonRegStat()),
        pytest.raises(ValueError, match="not a regular file"),
    ):
        read_wav(wav_path)


def test_read_wav_hostile_file_sizes(tmp_path: Path) -> None:
    tiny_wav = tmp_path / "tiny.wav"
    tiny_wav.write_bytes(b"RIFFshort")
    with pytest.raises(ValueError, match="smaller than RIFF header"):
        read_wav(tiny_wav)

    oversized_wav = tmp_path / "large.wav"
    oversized_wav.touch()

    class MockStat:
        st_size = MAX_WAV_BYTES + 100
        st_mode = stat.S_IFREG

    with (
        patch("os.fstat", return_value=MockStat()),
        pytest.raises(ValueError, match="exceeds MAX_WAV_BYTES"),
    ):
        read_wav(oversized_wav)


def test_read_wav_rejects_payload_overflow(tmp_path: Path) -> None:
    wav_path = tmp_path / "overflow.wav"
    wav_path.write_bytes(b"x" * 50)

    class MockFile:
        def __enter__(self) -> MockFile:
            return self

        def __exit__(self, *args: object) -> None:
            pass

        def read(self, n: int) -> bytes:
            return b"x" * (MAX_WAV_BYTES + 10)

    with (
        patch("os.fdopen", return_value=MockFile()),
        pytest.raises(ValueError, match="exceed MAX_WAV_BYTES"),
    ):
        read_wav(wav_path)


def test_splitmix64_pbt_roundtrip(tmp_path: Path) -> None:
    """SplitMix64 property-based testing of WAV encoding and reading."""
    rng = SplitMix64(0xCAFE_BABE_1234_5678)
    sample_rates = [8000.0, 11025.0, 16000.0, 22050.0, 44100.0, 48000.0, 96000.0]

    for iteration in range(30):
        sr = sample_rates[rng.next_int(0, len(sample_rates) - 1)]
        sample_count = rng.next_int(1, 500)
        samples = tuple(rng.next_int(-32768, 32767) for _ in range(sample_count))

        pcm = PCMBuffer(samples, PCMFormat(sample_rate_hz=sr))
        target_path = tmp_path / f"pbt_{iteration}.wav"

        written = write_wav(target_path, pcm)
        assert written == target_path

        recovered = read_wav(target_path)
        assert recovered.samples == samples
        assert recovered.pcm_format.sample_rate_hz == sr
        assert recovered.pcm_format.channel_count == 1
