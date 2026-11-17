"""Reusable WAV container sink for PCM audio buffers."""

from .wav_sink import (
    MAX_CHANNELS,
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    MAX_WAV_BYTES,
    OutputPath,
    calc_total_wav_bytes,
    check_wav_size_bounds,
    from_wav_bytes,
    read_wav,
    to_wav_bytes,
    validate_wav_path,
    write_wav,
)

__version__ = "0.1.0"

__all__ = [
    "MAX_CHANNELS",
    "MAX_DURATION_SECONDS",
    "MAX_SAMPLE_RATE_HZ",
    "MAX_SAMPLES",
    "MAX_WAV_BYTES",
    "OutputPath",
    "__version__",
    "calc_total_wav_bytes",
    "check_wav_size_bounds",
    "from_wav_bytes",
    "read_wav",
    "to_wav_bytes",
    "validate_wav_path",
    "write_wav",
]
