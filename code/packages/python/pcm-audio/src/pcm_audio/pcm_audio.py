"""Encode normalized floating samples into signed 16-bit PCM.

PCM is the first digital audio stage in the note-to-sound chain. It takes the
smooth world that the sampler observed and stores each value as a finite integer
that file formats, DACs, and device sinks can understand.
"""

from __future__ import annotations

import struct
from collections.abc import Iterable
from dataclasses import dataclass
from math import isfinite
from numbers import Integral, Real

from oscillator import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    SampleBuffer,
)

DEFAULT_SAMPLE_RATE_HZ = 44_100.0
DEFAULT_BIT_DEPTH = 16
DEFAULT_CHANNEL_COUNT = 1
DEFAULT_FULL_SCALE_VOLTAGE = 1.0
INTEGER_TOLERANCE = 1e-9
PCM16_MIN = -32_768
PCM16_MAX = 32_767

# CWE-400 resource exhaustion defense bounds
MAX_CHANNELS = 8
MAX_WAV_BYTES = 67_108_864  # 64 MiB


def _finite_float(name: str, value: Real) -> float:
    """Convert a finite real value to ``float`` and reject slippery inputs."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise ValueError(f"{name} must be a finite real number, got {value!r}")

    converted = float(value)
    if not isfinite(converted):
        raise ValueError(f"{name} must be finite, got {value!r}")

    return converted


def _positive_float(name: str, value: Real) -> float:
    converted = _finite_float(name, value)
    if converted <= 0.0:
        raise ValueError(f"{name} must be > 0.0, got {converted}")
    return converted


def _sample_rate(name: str, value: Real) -> float:
    converted = _positive_float(name, value)
    if converted > MAX_SAMPLE_RATE_HZ:
        raise ValueError(
            f"{name} {converted} exceeds MAX_SAMPLE_RATE_HZ={MAX_SAMPLE_RATE_HZ}"
        )
    return converted


def _non_negative_int(name: str, value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, Integral):
        raise ValueError(f"{name} must be an integer >= 0, got {value!r}")
    converted = int(value)
    if converted < 0:
        raise ValueError(f"{name} must be >= 0, got {converted}")
    return converted


def _positive_int(name: str, value: int) -> int:
    converted = _non_negative_int(name, value)
    if converted == 0:
        raise ValueError(f"{name} must be > 0, got 0")
    return converted


def _integer_sample_rate(sample_rate_hz: float) -> int:
    rounded = round(sample_rate_hz)
    if abs(sample_rate_hz - rounded) > INTEGER_TOLERANCE:
        raise ValueError(
            "sample_rate_hz must be an integer-valued rate for WAV output, "
            f"got {sample_rate_hz}"
        )
    return int(rounded)


def _safe_round_sample(sample: Real) -> tuple[int, bool]:
    """
    Safely quantize a normalized floating-point sample into signed 16-bit PCM.

    Mathematical specification from MUS01 §18 & CWE-190/CWE-20 hardening:
    - x in [0.0, 1.0] -> round(x * 32767)
    - x in [-1.0, 0.0) -> round(x * 32768)
    - Values outside [-1.0, 1.0] are clamped to [-1.0, 1.0] and flagged clipped.
    - Non-finite numbers (NaN, ±Inf) and booleans are rejected fail-closed.
    """
    if isinstance(sample, bool) or not isinstance(sample, Real):
        raise ValueError(f"sample must be a finite real number, got {sample!r}")

    val = float(sample)
    if not isfinite(val):
        raise ValueError(f"sample must be finite, got {sample!r}")

    clipped = val < -1.0 or val > 1.0
    bounded = min(1.0, max(-1.0, val))

    if bounded >= 0.0:
        raw = int(round(bounded * PCM16_MAX))
    else:
        raw = int(round(bounded * abs(PCM16_MIN)))

    clamped = max(PCM16_MIN, min(PCM16_MAX, raw))
    return clamped, clipped


float_to_pcm16 = _safe_round_sample
safe_round_sample = _safe_round_sample


@dataclass(frozen=True)
class PCMFormat:
    """Metadata for the first digital audio representation: mono 16-bit PCM."""

    sample_rate_hz: float = DEFAULT_SAMPLE_RATE_HZ
    channel_count: int = DEFAULT_CHANNEL_COUNT
    bit_depth: int = DEFAULT_BIT_DEPTH
    full_scale_voltage: float = DEFAULT_FULL_SCALE_VOLTAGE

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "sample_rate_hz",
            _sample_rate("sample_rate_hz", self.sample_rate_hz),
        )
        channel_count = _positive_int("channel_count", self.channel_count)
        if channel_count > MAX_CHANNELS:
            raise ValueError(
                f"channel_count {channel_count} exceeds MAX_CHANNELS={MAX_CHANNELS}"
            )
        if channel_count != DEFAULT_CHANNEL_COUNT:
            raise ValueError("only mono PCM is supported in V1")
        object.__setattr__(self, "channel_count", channel_count)

        bit_depth = _positive_int("bit_depth", self.bit_depth)
        if bit_depth != DEFAULT_BIT_DEPTH:
            raise ValueError("only signed 16-bit PCM is supported in V1")
        object.__setattr__(self, "bit_depth", bit_depth)
        object.__setattr__(
            self,
            "full_scale_voltage",
            _positive_float("full_scale_voltage", self.full_scale_voltage),
        )

    @property
    def minimum_integer(self) -> int:
        """Return the lowest signed PCM integer for this format."""
        return PCM16_MIN

    @property
    def maximum_integer(self) -> int:
        """Return the highest signed PCM integer for this format."""
        return PCM16_MAX

    @property
    def sample_width_bytes(self) -> int:
        """Return the number of bytes per PCM sample."""
        return self.bit_depth // 8

    def integer_sample_rate(self) -> int:
        """Return a WAV-compatible integer sample rate."""
        return _integer_sample_rate(self.sample_rate_hz)


@dataclass(frozen=True)
class PCMBuffer:
    """PCM samples plus the timing metadata needed by DAC and file sinks."""

    samples: tuple[int, ...]
    pcm_format: PCMFormat
    start_time_seconds: float = 0.0
    clipped_sample_count: int = 0

    def __post_init__(self) -> None:
        if len(self.samples) > MAX_SAMPLES:
            raise ValueError(
                f"sample count {len(self.samples)} exceeds MAX_SAMPLES={MAX_SAMPLES}"
            )

        checked_samples: list[int] = []
        for index, sample in enumerate(self.samples):
            if isinstance(sample, bool) or not isinstance(sample, Integral):
                raise ValueError(f"samples[{index}] must be a PCM integer")
            converted = int(sample)
            if converted < PCM16_MIN or converted > PCM16_MAX:
                raise ValueError(
                    f"samples[{index}] must fit signed 16-bit PCM, got {converted}"
                )
            checked_samples.append(converted)

        object.__setattr__(self, "samples", tuple(checked_samples))
        if not isinstance(self.pcm_format, PCMFormat):
            raise ValueError("pcm_format must be a PCMFormat")
        object.__setattr__(
            self,
            "start_time_seconds",
            _finite_float("start_time_seconds", self.start_time_seconds),
        )
        object.__setattr__(
            self,
            "clipped_sample_count",
            _non_negative_int("clipped_sample_count", self.clipped_sample_count),
        )

    def sample_count(self) -> int:
        """Return the number of PCM samples."""
        return len(self.samples)

    def sample_period_seconds(self) -> float:
        """Return the time spacing between adjacent samples."""
        return 1.0 / self.pcm_format.sample_rate_hz

    def duration_seconds(self) -> float:
        """Return the half-open duration covered by this PCM buffer."""
        return self.sample_count() / self.pcm_format.sample_rate_hz

    def time_at(self, index: int) -> float:
        """Return the timestamp for a PCM sample index."""
        if isinstance(index, bool) or not isinstance(index, Integral):
            raise ValueError(f"index must be an integer, got {index!r}")
        converted = int(index)
        if not 0 <= converted < self.sample_count():
            raise ValueError(
                f"index must be in [0, {self.sample_count()}), got {converted}"
            )
        return self.start_time_seconds + converted / self.pcm_format.sample_rate_hz

    def to_little_endian_bytes(self) -> bytes:
        """Pack signed 16-bit PCM integers as little-endian bytes."""
        byte_length = len(self.samples) * 2
        if byte_length > MAX_WAV_BYTES:
            raise ValueError(
                f"byte length {byte_length} exceeds MAX_WAV_BYTES={MAX_WAV_BYTES}"
            )
        return b"".join(
            sample.to_bytes(2, "little", signed=True) for sample in self.samples
        )


def encode_sample_buffer(
    sample_buffer: SampleBuffer,
    pcm_format: PCMFormat | None = None,
) -> PCMBuffer:
    """Quantize floating oscillator samples into signed 16-bit PCM."""
    if not isinstance(sample_buffer, SampleBuffer):
        raise ValueError("sample_buffer must be an oscillator.SampleBuffer")

    if sample_buffer.sample_count() > MAX_SAMPLES:
        raise ValueError(
            f"sample count {sample_buffer.sample_count()} exceeds "
            f"MAX_SAMPLES={MAX_SAMPLES}"
        )

    active_format = (
        pcm_format
        if pcm_format is not None
        else PCMFormat(sample_rate_hz=sample_buffer.sample_rate_hz)
    )

    pcm_samples: list[int] = []
    clipped_count = 0
    for sample in sample_buffer.samples:
        pcm_sample, clipped = _safe_round_sample(sample)
        pcm_samples.append(pcm_sample)
        if clipped:
            clipped_count += 1

    return PCMBuffer(
        samples=tuple(pcm_samples),
        pcm_format=active_format,
        start_time_seconds=sample_buffer.start_time_seconds,
        clipped_sample_count=clipped_count,
    )


def samples_to_pcm_buffer(
    samples: Iterable[Real],
    *,
    sample_rate_hz: Real,
    start_time_seconds: Real = 0.0,
    pcm_format: PCMFormat | None = None,
) -> PCMBuffer:
    """Convenience helper for demos that start with raw floating samples."""
    rate = _sample_rate("sample_rate_hz", sample_rate_hz)
    start = _finite_float("start_time_seconds", start_time_seconds)

    collected: list[float] = []
    for index, sample in enumerate(samples):
        if len(collected) >= MAX_SAMPLES:
            raise ValueError(f"samples sequence exceeds MAX_SAMPLES={MAX_SAMPLES}")
        collected.append(_finite_float(f"samples[{index}]", sample))

    sample_buffer = SampleBuffer(
        samples=tuple(collected),
        sample_rate_hz=rate,
        start_time_seconds=start,
    )
    return encode_sample_buffer(sample_buffer, pcm_format)


def validate_wav_header(wav_bytes: bytes) -> tuple[int, int, int, int, int]:
    """
    Validate WAV container headers fail-closed against CWE-400 and CWE-20.

    Returns:
        (channels, sample_rate, bit_depth, data_offset, data_length)
    """
    if not isinstance(wav_bytes, (bytes, bytearray)):
        raise ValueError(f"wav_bytes must be bytes, got {type(wav_bytes).__name__}")

    total_len = len(wav_bytes)
    if total_len > MAX_WAV_BYTES:
        raise ValueError(
            f"WAV byte length {total_len} exceeds MAX_WAV_BYTES={MAX_WAV_BYTES}"
        )
    if total_len < 44:
        raise ValueError(
            f"WAV byte length {total_len} is smaller than minimum header size 44"
        )

    if wav_bytes[:4] != b"RIFF":
        raise ValueError("missing 'RIFF' container magic")
    if wav_bytes[8:12] != b"WAVE":
        raise ValueError("missing 'WAVE' format marker")

    riff_payload = int.from_bytes(wav_bytes[4:8], "little")
    if riff_payload < 36:
        raise ValueError(f"RIFF payload size too small: {riff_payload}")

    offset = 12
    fmt_parsed = False
    channels = 0
    sample_rate = 0
    bits_per_sample = 0
    data_offset = 0
    data_len = 0

    while offset + 8 <= total_len:
        chunk_id = bytes(wav_bytes[offset : offset + 4])
        chunk_size = int.from_bytes(wav_bytes[offset + 4 : offset + 8], "little")
        data_start = offset + 8
        data_end = data_start + chunk_size

        if data_end > total_len:
            raise ValueError(
                f"chunk {chunk_id!r} declared size {chunk_size} exceeds buffer"
            )

        if chunk_id == b"fmt ":
            if chunk_size < 16:
                raise ValueError("fmt chunk size too small")
            audio_format = int.from_bytes(
                wav_bytes[data_start : data_start + 2], "little"
            )
            if audio_format not in (1, 3):  # 1 = PCM, 3 = IEEE float
                raise ValueError(f"unsupported WAV audio format {audio_format}")
            channels = int.from_bytes(
                wav_bytes[data_start + 2 : data_start + 4], "little"
            )
            if channels == 0 or channels > MAX_CHANNELS:
                raise ValueError(f"unsupported channel count {channels}")
            sample_rate = int.from_bytes(
                wav_bytes[data_start + 4 : data_start + 8], "little"
            )
            if sample_rate == 0 or sample_rate > MAX_SAMPLE_RATE_HZ:
                raise ValueError(
                    f"sample rate {sample_rate} outside (0, {MAX_SAMPLE_RATE_HZ}]"
                )
            byte_rate = int.from_bytes(
                wav_bytes[data_start + 8 : data_start + 12], "little"
            )
            block_align = int.from_bytes(
                wav_bytes[data_start + 12 : data_start + 14], "little"
            )
            bits_per_sample = int.from_bytes(
                wav_bytes[data_start + 14 : data_start + 16], "little"
            )

            if audio_format == 1 and bits_per_sample != 16:
                raise ValueError(f"unsupported bit depth {bits_per_sample} for PCM")
            if audio_format == 3 and bits_per_sample != 32:
                raise ValueError(
                    f"unsupported bit depth {bits_per_sample} for IEEE float"
                )

            expected_block_align = channels * (bits_per_sample // 8)
            if block_align != expected_block_align:
                raise ValueError(
                    f"inconsistent block align {block_align} != {expected_block_align}"
                )
            expected_byte_rate = sample_rate * block_align
            if byte_rate != expected_byte_rate:
                raise ValueError(
                    f"inconsistent byte rate {byte_rate} != {expected_byte_rate}"
                )
            fmt_parsed = True
        elif chunk_id == b"data":
            if not fmt_parsed:
                raise ValueError("data chunk before fmt chunk")
            if chunk_size % block_align != 0:
                raise ValueError(
                    f"data chunk size {chunk_size} is not a multiple of "
                    f"block align {block_align}"
                )
            data_offset = data_start
            data_len = chunk_size
            break

        pad = chunk_size % 2
        offset = data_end + pad

    if not fmt_parsed:
        raise ValueError("missing fmt chunk in WAV")
    if data_offset == 0:
        raise ValueError("missing data chunk in WAV")

    return channels, sample_rate, bits_per_sample, data_offset, data_len


def from_wav_bytes(wav_bytes: bytes) -> PCMBuffer:
    """Parse RIFF/WAVE bytes into a PCMBuffer."""
    channels, sample_rate, bits_per_sample, data_offset, data_len = validate_wav_header(
        wav_bytes
    )

    if channels != 1:
        raise ValueError(f"only mono WAV files are supported, got {channels} channels")

    bytes_per_sample = bits_per_sample // 8
    sample_count = data_len // bytes_per_sample
    if sample_count > MAX_SAMPLES:
        raise ValueError(
            f"sample count {sample_count} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )

    raw_data = wav_bytes[data_offset : data_offset + data_len]
    samples: list[int] = []
    clipped_count = 0

    if bits_per_sample == 16:
        # 16-bit signed PCM
        for i in range(0, data_len, 2):
            val = struct.unpack_from("<h", raw_data, i)[0]
            samples.append(val)
    elif bits_per_sample == 32:
        # 32-bit float PCM -> quantize to 16-bit PCM
        for i in range(0, data_len, 4):
            fval = struct.unpack_from("<f", raw_data, i)[0]
            val, clipped = _safe_round_sample(fval)
            samples.append(val)
            if clipped:
                clipped_count += 1

    fmt = PCMFormat(
        sample_rate_hz=float(sample_rate),
        channel_count=channels,
        bit_depth=16,
    )
    return PCMBuffer(
        samples=tuple(samples),
        pcm_format=fmt,
        start_time_seconds=0.0,
        clipped_sample_count=clipped_count,
    )


__all__ = [
    "DEFAULT_BIT_DEPTH",
    "DEFAULT_CHANNEL_COUNT",
    "DEFAULT_FULL_SCALE_VOLTAGE",
    "DEFAULT_SAMPLE_RATE_HZ",
    "INTEGER_TOLERANCE",
    "MAX_CHANNELS",
    "MAX_DURATION_SECONDS",
    "MAX_SAMPLES",
    "MAX_SAMPLE_RATE_HZ",
    "MAX_WAV_BYTES",
    "PCM16_MAX",
    "PCM16_MIN",
    "PCMBuffer",
    "PCMFormat",
    "_safe_round_sample",
    "encode_sample_buffer",
    "float_to_pcm16",
    "from_wav_bytes",
    "safe_round_sample",
    "samples_to_pcm_buffer",
    "validate_wav_header",
]
