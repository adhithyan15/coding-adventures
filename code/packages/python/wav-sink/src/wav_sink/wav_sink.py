"""Serialize PCM buffers into deterministic RIFF/WAVE bytes.

Provides deterministic WAV serialization and file sink operations with
strict CWE-400 (resource exhaustion), CWE-22 (path traversal / symlink),
and CWE-20 (input validation) protections.
"""

from __future__ import annotations

import math
import os
import stat
import wave
from io import BytesIO
from os import PathLike
from pathlib import Path

from oscillator import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
)
from pcm_audio import (
    MAX_CHANNELS,
    MAX_WAV_BYTES,
    PCMBuffer,
    from_wav_bytes,
)

OutputPath = str | PathLike[str]


def calc_total_wav_bytes(payload: int) -> int:
    """Calculate total WAV container bytes for a given PCM payload length.

    A standard canonical RIFF/WAVE header occupies exactly 44 bytes.
    """
    if isinstance(payload, bool) or not isinstance(payload, int):
        raise TypeError(f"payload must be an int, got {type(payload).__name__}")
    if payload < 0:
        raise ValueError(f"payload must be non-negative, got {payload}")
    return 44 + payload


def check_wav_size_bounds(size: int) -> bool:
    """Check if total WAV byte size is within valid bounds [44, MAX_WAV_BYTES]."""
    if isinstance(size, bool) or not isinstance(size, int):
        return False
    return 44 <= size <= MAX_WAV_BYTES


def _check_no_symlinks(target_path: Path) -> None:
    """Verify that neither the target nor any parent directory is a symlink."""
    if target_path.is_symlink():
        raise ValueError(f"Path '{target_path}' is a symlink")

    curr = target_path.parent
    visited: set[Path] = set()
    while curr != curr.parent and curr not in visited:
        visited.add(curr)
        if curr.is_symlink():
            raise ValueError(f"Path ancestor '{curr}' is a symlink")
        curr = curr.parent


def validate_wav_path(
    path: OutputPath,
    base_dir: Path | str | None = None,
    must_exist: bool = False,
) -> Path:
    """Validate output/input path for WAV files (CWE-22 defense).

    Enforces:
    - Path must be str or PathLike, non-empty, and free of null bytes.
    - Rejects path traversal ('..') in path parts unconditionally.
    - Extension must be .wav or .wave (case-insensitive).
    - If base_dir is supplied:
      - Symlinks on base_dir are checked BEFORE resolving.
      - Path traversal ('..') in base_dir is rejected.
      - The resolved target path must be contained strictly within the base_dir.
    - Neither target nor any ancestor path may be a symlink.
    - If must_exist is True, target must exist and be a regular file.
    - If must_exist is False and target exists, target must be a regular file
      and never a symlink (prevent overwriting symlinks).
    """
    if isinstance(path, bool) or not isinstance(path, (str, PathLike)):
        raise TypeError(f"path must be a str or PathLike, got {type(path).__name__}")

    str_path = str(path)
    if "\0" in str_path:
        raise ValueError("Path must not contain null bytes")
    if not str_path.strip():
        raise ValueError("Path must not be empty")

    p = Path(path)
    if ".." in p.parts:
        raise ValueError(f"Path traversal ('..') detected in path: '{path}'")

    if p.suffix.lower() not in (".wav", ".wave"):
        raise ValueError(f"Path must end in .wav or .wave, got '{p.suffix}'")

    if base_dir is not None:
        if isinstance(base_dir, bool) or not isinstance(base_dir, (str, PathLike)):
            raise TypeError(
                f"base_dir must be a str or PathLike, got {type(base_dir).__name__}"
            )
        str_base = str(base_dir)
        if "\0" in str_base:
            raise ValueError("base_dir must not contain null bytes")
        base_raw = Path(base_dir)
        if ".." in base_raw.parts:
            raise ValueError(
                f"Path traversal ('..') detected in base_dir: '{base_dir}'"
            )
        if not base_raw.exists():
            raise ValueError(f"base_dir '{base_dir}' does not exist")
        if not base_raw.is_dir():
            raise ValueError(f"base_dir '{base_dir}' is not a directory")
        _check_no_symlinks(base_raw)
        base_path = base_raw.resolve()

        _check_no_symlinks(p)
        resolved_target = p.resolve()
        try:
            resolved_target.relative_to(base_path)
        except ValueError as err:
            raise ValueError(
                f"Path '{path}' escapes base directory '{base_dir}'"
            ) from err
    else:
        _check_no_symlinks(p)

    if must_exist:
        if not p.exists():
            raise FileNotFoundError(f"WAV file '{path}' does not exist")
        if not p.is_file():
            raise ValueError(f"WAV path '{path}' is not a regular file")
    else:
        parent = p.parent
        if parent.exists() and not parent.is_dir():
            raise ValueError(f"Parent '{parent}' is not a directory")
        if p.exists() and not p.is_file():
            raise ValueError(f"Target '{path}' exists and is not a regular file")

    return p


def to_wav_bytes(pcm_buffer: PCMBuffer) -> bytes:
    """Write a mono signed-16-bit PCM buffer into a deterministic WAV container."""
    if not isinstance(pcm_buffer, PCMBuffer):
        raise ValueError("pcm_buffer must be a PCMBuffer")

    raw_count = pcm_buffer.sample_count
    sample_count = raw_count() if callable(raw_count) else raw_count
    if not isinstance(sample_count, int) or isinstance(sample_count, bool):
        sample_count = len(pcm_buffer.samples)
    if sample_count > MAX_SAMPLES:
        raise ValueError(
            f"sample count {sample_count} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )

    channel_count = pcm_buffer.pcm_format.channel_count
    if channel_count < 1 or channel_count > MAX_CHANNELS:
        raise ValueError(
            f"channel count {channel_count} out of range [1, {MAX_CHANNELS}]"
        )

    sample_width = pcm_buffer.pcm_format.sample_width_bytes
    if sample_width != 2:
        raise ValueError(
            f"unsupported sample width {sample_width} bytes; "
            "only 16-bit PCM (2 bytes) is supported"
        )

    sample_rate_hz = pcm_buffer.pcm_format.sample_rate_hz
    if isinstance(sample_rate_hz, bool) or not isinstance(sample_rate_hz, (int, float)):
        raise TypeError("sample_rate_hz must be numeric")
    if math.isnan(sample_rate_hz) or math.isinf(sample_rate_hz):
        raise ValueError("sample_rate_hz must not be NaN or Inf")
    if sample_rate_hz <= 0.0 or sample_rate_hz > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"sample_rate_hz must be between 1 and {MAX_SAMPLE_RATE_HZ}, "
            f"got {sample_rate_hz}"
        )

    sample_rate = pcm_buffer.pcm_format.integer_sample_rate()
    if sample_rate <= 0 or sample_rate > int(MAX_SAMPLE_RATE_HZ):
        raise ValueError(
            f"integer sample rate must be between 1 and {int(MAX_SAMPLE_RATE_HZ)}"
        )

    frames = sample_count // channel_count if channel_count > 0 else sample_count
    duration = frames / sample_rate_hz
    if duration > MAX_DURATION_SECONDS:
        raise ValueError(
            f"duration {duration:.3f}s exceeds "
            f"MAX_DURATION_SECONDS ({MAX_DURATION_SECONDS}s)"
        )

    payload_size = sample_count * sample_width
    total_wav_size = calc_total_wav_bytes(payload_size)
    if not check_wav_size_bounds(total_wav_size):
        raise ValueError(
            f"total WAV bytes {total_wav_size} exceeds MAX_WAV_BYTES ({MAX_WAV_BYTES})"
        )

    raw_frames = pcm_buffer.to_little_endian_bytes()
    output = BytesIO()
    with wave.open(output, "wb") as wav_file:
        wav_file.setnchannels(channel_count)
        wav_file.setsampwidth(sample_width)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(raw_frames)

    result = output.getvalue()
    if len(result) > MAX_WAV_BYTES:
        raise ValueError(
            f"serialized WAV size ({len(result)}) exceeds "
            f"MAX_WAV_BYTES ({MAX_WAV_BYTES})"
        )
    return result


def write_wav(
    path: OutputPath,
    pcm_buffer: PCMBuffer,
    base_dir: Path | str | None = None,
) -> Path:
    """Write a WAV file safely and return the exact ``Path`` object used."""
    validated_path = validate_wav_path(path, base_dir=base_dir, must_exist=False)
    wav_bytes = to_wav_bytes(pcm_buffer)

    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_TRUNC
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_BINARY", 0)
    )
    fd = os.open(validated_path, flags, 0o644)
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode):
            raise ValueError(f"WAV target '{path}' is not a regular file")
        with os.fdopen(fd, "wb") as f:
            fd = -1
            f.write(wav_bytes)
    finally:
        if fd >= 0:
            os.close(fd)

    return validated_path


def read_wav(
    path: OutputPath,
    base_dir: Path | str | None = None,
) -> PCMBuffer:
    """Read a WAV file safely and parse it into a PCMBuffer."""
    validated_path = validate_wav_path(path, base_dir=base_dir, must_exist=True)

    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_BINARY", 0)
    fd = os.open(validated_path, flags)
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode):
            raise ValueError(f"WAV path '{path}' is not a regular file")
        if st.st_size < 44:
            raise ValueError(
                f"WAV file size ({st.st_size} bytes) is smaller than "
                "RIFF header (44 bytes)"
            )
        if st.st_size > MAX_WAV_BYTES:
            raise ValueError(
                f"WAV file size ({st.st_size} bytes) exceeds "
                f"MAX_WAV_BYTES ({MAX_WAV_BYTES})"
            )
        with os.fdopen(fd, "rb") as f:
            fd = -1
            wav_bytes = f.read(MAX_WAV_BYTES + 1)
        if len(wav_bytes) > MAX_WAV_BYTES:
            raise ValueError(
                f"Read bytes ({len(wav_bytes)}) exceed MAX_WAV_BYTES ({MAX_WAV_BYTES})"
            )
        return from_wav_bytes(wav_bytes)
    finally:
        if fd >= 0:
            os.close(fd)
