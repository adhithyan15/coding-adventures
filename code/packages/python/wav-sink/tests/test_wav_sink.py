"""Tests for the reusable WAV sink."""

from __future__ import annotations

import wave
from io import BytesIO
from pathlib import Path

import pytest
from pcm_audio import PCMBuffer, PCMFormat

from wav_sink import __version__, to_wav_bytes, write_wav


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_wav_bytes_are_parseable_mono_pcm() -> None:
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


def test_write_wav_writes_the_same_bytes(tmp_path: Path) -> None:
    pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=8.0))
    output_path = tmp_path / "tone.wav"

    returned_path = write_wav(output_path, pcm)

    assert returned_path == output_path
    assert output_path.read_bytes() == to_wav_bytes(pcm)


def test_wav_output_rejects_bad_buffers() -> None:
    with pytest.raises(ValueError, match="pcm_buffer"):
        to_wav_bytes(object())  # type: ignore[arg-type]


def test_wav_output_rejects_non_integer_sample_rates() -> None:
    pcm = PCMBuffer((0,), PCMFormat(sample_rate_hz=44_100.5))

    with pytest.raises(ValueError, match="integer-valued"):
        to_wav_bytes(pcm)
