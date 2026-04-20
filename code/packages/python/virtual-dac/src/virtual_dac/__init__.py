"""Reusable virtual DAC stage for PCM audio pipelines."""

from .virtual_dac import ZeroOrderHoldDACSignal, pcm16_to_voltage

__version__ = "0.1.0"

__all__ = [
    "ZeroOrderHoldDACSignal",
    "__version__",
    "pcm16_to_voltage",
]
