"""Serialize PCM buffers into deterministic RIFF/WAVE bytes."""

from __future__ import annotations

import wave
from io import BytesIO
from os import PathLike
from pathlib import Path

from pcm_audio import PCMBuffer

OutputPath = str | PathLike[str]


def to_wav_bytes(pcm_buffer: PCMBuffer) -> bytes:
    """Write a mono signed-16-bit PCM buffer into a deterministic WAV container."""

    if not isinstance(pcm_buffer, PCMBuffer):
        raise ValueError("pcm_buffer must be a PCMBuffer")

    sample_rate = pcm_buffer.pcm_format.integer_sample_rate()
    output = BytesIO()
    with wave.open(output, "wb") as wav_file:
        wav_file.setnchannels(pcm_buffer.pcm_format.channel_count)
        wav_file.setsampwidth(pcm_buffer.pcm_format.sample_width_bytes)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(pcm_buffer.to_little_endian_bytes())
    return output.getvalue()


def write_wav(path: OutputPath, pcm_buffer: PCMBuffer) -> Path:
    """Write a WAV file and return the exact ``Path`` object used."""

    output_path = Path(path)
    output_path.write_bytes(to_wav_bytes(pcm_buffer))
    return output_path
