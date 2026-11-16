"""Tests for the reusable virtual speaker stage."""

from __future__ import annotations

import math
from dataclasses import dataclass
from numbers import Real

import pytest

from virtual_speaker import (
    DEFAULT_SPEAKER_GAIN,
    MAX_SPEAKER_GAIN,
    AnalogSignal,
    LinearSpeakerSignal,
    __version__,
)

# =============================================================================
# § 1. Pure Python SplitMix64 Deterministic PRNG for PBT / Fuzzing
# =============================================================================


class SplitMix64:
    """Deterministic 64-bit PRNG (Steele et al., 2014) for reproducible PBT."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFF_FFFF_FFFF_FFFF
        return z ^ (z >> 31)

    def next_float(self) -> float:
        """Return uniform float in [0.0, 1.0)."""
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)

    def uniform(self, low: float, high: float) -> float:
        """Return uniform float in [low, high)."""
        return low + (high - low) * self.next_float()


@dataclass(frozen=True)
class ConstantSignal:
    """Tiny test signal that always returns the same value."""

    value: float

    def value_at(self, time_seconds: Real) -> float:
        return self.value + float(time_seconds) * 0.0


def test_version_and_defaults_are_visible() -> None:
    assert __version__ == "0.1.0"
    assert DEFAULT_SPEAKER_GAIN == 1.0
    assert MAX_SPEAKER_GAIN == 1_000_000.0


def test_constant_signal_satisfies_protocol() -> None:
    signal: AnalogSignal = ConstantSignal(0.5)

    assert signal.value_at(10.0) == 0.5


def test_linear_speaker_gain_scales_input_signal() -> None:
    speaker = LinearSpeakerSignal(ConstantSignal(0.5), speaker_gain=2.0)

    assert speaker.value_at(0.0) == 1.0
    assert speaker.value_at(10.0) == 1.0


def test_default_gain_is_unity() -> None:
    speaker = LinearSpeakerSignal(ConstantSignal(-0.25))

    assert speaker.value_at(0.0) == -0.25


def test_rejects_invalid_speaker_gain() -> None:
    with pytest.raises(ValueError, match="speaker_gain"):
        LinearSpeakerSignal(ConstantSignal(0.0), speaker_gain=math.nan)
    with pytest.raises(ValueError, match="speaker_gain"):
        LinearSpeakerSignal(ConstantSignal(0.0), speaker_gain=True)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="MAX_SPEAKER_GAIN"):
        LinearSpeakerSignal(ConstantSignal(0.0), speaker_gain=MAX_SPEAKER_GAIN + 1.0)


def test_rejects_invalid_time_values() -> None:
    speaker = LinearSpeakerSignal(ConstantSignal(0.0))

    with pytest.raises(ValueError, match="time_seconds"):
        speaker.value_at(math.inf)
    with pytest.raises(ValueError, match="time_seconds"):
        speaker.value_at(True)  # type: ignore[arg-type]


def test_rejects_objects_without_value_at() -> None:
    with pytest.raises(ValueError, match="analog_signal"):
        LinearSpeakerSignal(object())  # type: ignore[arg-type]


def test_rejects_signal_returning_non_finite_or_bool() -> None:
    @dataclass(frozen=True)
    class HostileSignal:
        bad_val: object

        def value_at(self, time_seconds: Real) -> float:
            return self.bad_val  # type: ignore[return-value]

    nan_speaker = LinearSpeakerSignal(HostileSignal(math.nan))
    with pytest.raises(ValueError, match="must be finite"):
        nan_speaker.value_at(0.0)

    bool_speaker = LinearSpeakerSignal(HostileSignal(True))
    with pytest.raises(ValueError, match="must be a finite real number"):
        bool_speaker.value_at(0.0)


# =============================================================================
# § 2. Property-Based Testing (PBT) using SplitMix64 PRNG
# =============================================================================


def test_splitmix64_pbt_linear_speaker_gain_exactness() -> None:
    """PBT: Speaker output is exactly gain * input across random gains and voltages."""
    prng = SplitMix64(seed=0x9999_8888_7777_6666)

    for _ in range(50):
        v = prng.uniform(-10.0, 10.0)
        gain = prng.uniform(-100.0, 100.0)
        speaker = LinearSpeakerSignal(ConstantSignal(v), speaker_gain=gain)

        for _ in range(10):
            t = prng.uniform(0.0, 10.0)
            res = speaker.value_at(t)
            expected = gain * v
            assert abs(res - expected) <= 1e-9
