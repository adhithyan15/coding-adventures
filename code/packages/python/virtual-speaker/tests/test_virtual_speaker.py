"""Tests for the reusable virtual speaker stage."""

from __future__ import annotations

import math
from dataclasses import dataclass
from numbers import Real

import pytest

from virtual_speaker import (
    DEFAULT_SPEAKER_GAIN,
    AnalogSignal,
    LinearSpeakerSignal,
    __version__,
)


@dataclass(frozen=True)
class ConstantSignal:
    """Tiny test signal that always returns the same value."""

    value: float

    def value_at(self, time_seconds: Real) -> float:
        return self.value + float(time_seconds) * 0.0


def test_version_and_defaults_are_visible() -> None:
    assert __version__ == "0.1.0"
    assert DEFAULT_SPEAKER_GAIN == 1.0


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


def test_rejects_invalid_time_values() -> None:
    speaker = LinearSpeakerSignal(ConstantSignal(0.0))

    with pytest.raises(ValueError, match="time_seconds"):
        speaker.value_at(math.inf)
    with pytest.raises(ValueError, match="time_seconds"):
        speaker.value_at(True)  # type: ignore[arg-type]


def test_rejects_objects_without_value_at() -> None:
    with pytest.raises(ValueError, match="analog_signal"):
        LinearSpeakerSignal(object())  # type: ignore[arg-type]
