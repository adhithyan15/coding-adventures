"""Reusable virtual DAC stage for PCM audio pipelines."""

from .virtual_dac import (
    MAX_DURATION_SECONDS,
    MAX_SAMPLE_RATE_HZ,
    MAX_SAMPLES,
    ZeroOrderHoldDACSignal,
    pcm16_to_voltage,
)

__version__ = "0.1.0"

__all__ = [
    "MAX_DURATION_SECONDS",
    "MAX_SAMPLES",
    "MAX_SAMPLE_RATE_HZ",
    "ZeroOrderHoldDACSignal",
    "__version__",
    "pcm16_to_voltage",
]
