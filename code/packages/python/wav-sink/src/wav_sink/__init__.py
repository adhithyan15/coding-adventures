"""Reusable WAV container sink for PCM audio buffers."""

from .wav_sink import OutputPath, to_wav_bytes, write_wav

__version__ = "0.1.0"

__all__ = [
    "OutputPath",
    "__version__",
    "to_wav_bytes",
    "write_wav",
]
