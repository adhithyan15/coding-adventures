"""Reusable PCM encoding stage for virtual audio pipelines."""

from .pcm_audio import (
    DEFAULT_BIT_DEPTH,
    DEFAULT_CHANNEL_COUNT,
    DEFAULT_FULL_SCALE_VOLTAGE,
    DEFAULT_SAMPLE_RATE_HZ,
    PCM16_MAX,
    PCM16_MIN,
    PCMBuffer,
    PCMFormat,
    encode_sample_buffer,
    float_to_pcm16,
    samples_to_pcm_buffer,
)

__version__ = "0.1.0"

__all__ = [
    "DEFAULT_BIT_DEPTH",
    "DEFAULT_CHANNEL_COUNT",
    "DEFAULT_FULL_SCALE_VOLTAGE",
    "DEFAULT_SAMPLE_RATE_HZ",
    "PCM16_MAX",
    "PCM16_MIN",
    "PCMBuffer",
    "PCMFormat",
    "__version__",
    "encode_sample_buffer",
    "float_to_pcm16",
    "samples_to_pcm_buffer",
]
