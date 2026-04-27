"""Deterministic naive unpitched percussion voices and kits."""

from .percussion_kit import (
    DEFAULT_MAX_MODE_COUNT,
    DEFAULT_MAX_SAMPLE_COUNT,
    DEFAULT_SAMPLE_RATE_HZ,
    DrumKitProfile,
    PercussionEnvelope,
    PercussionHitRender,
    PercussionMode,
    PercussionNoiseProfile,
    PercussionSignal,
    PercussionVoiceProfile,
    all_drum_kits,
    all_standard_hit_ids,
    get_drum_kit,
    get_percussion_voice,
    render_percussion_hit,
    standard_kit,
)

__version__ = "0.1.0"

__all__ = [
    "DEFAULT_MAX_MODE_COUNT",
    "DEFAULT_MAX_SAMPLE_COUNT",
    "DEFAULT_SAMPLE_RATE_HZ",
    "DrumKitProfile",
    "PercussionEnvelope",
    "PercussionHitRender",
    "PercussionMode",
    "PercussionNoiseProfile",
    "PercussionSignal",
    "PercussionVoiceProfile",
    "__version__",
    "all_drum_kits",
    "all_standard_hit_ids",
    "get_drum_kit",
    "get_percussion_voice",
    "render_percussion_hit",
    "standard_kit",
]
