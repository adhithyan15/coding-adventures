"""Reusable virtual speaker stage for audio pipelines."""

from .virtual_speaker import (
    DEFAULT_SPEAKER_GAIN,
    AnalogSignal,
    LinearSpeakerSignal,
)

__version__ = "0.1.0"

__all__ = [
    "DEFAULT_SPEAKER_GAIN",
    "AnalogSignal",
    "LinearSpeakerSignal",
    "__version__",
]
