"""Opt-in finite and failure-atomic binary64 Biquad/SOS filtering.

The legacy :mod:`dsp_filters.biquad` entry points intentionally keep their
historical coercion, reciprocal normalization, and partial-progress behavior.
This module provides a separate checked path for callers that need strict
primitive inputs and state rollback on ordinary returned failures.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_filters.fir import (
    MAX_ORDER,
    MAX_SAMPLES,
    EmptySignalError,
    InvalidCoefficientError,
)

type _Sequence = Sequence[object]


def _finite_number(value: object, name: str) -> float:
    """Accept exact built-in numeric types and return one binary64 value."""
    if type(value) is float:
        converted = value
    elif type(value) is int:
        try:
            converted = float(value)
        except OverflowError as exc:
            raise InvalidCoefficientError(f"{name} is outside binary64 range") from exc
    else:
        raise InvalidCoefficientError(f"{name} must be a built-in int or float")

    if not math.isfinite(converted):
        raise InvalidCoefficientError(f"{name} must be finite")
    return converted


def _length_once(value: object, name: str) -> int:
    """Read a caller-provided length once and enforce safe bounded indexing."""
    if not isinstance(value, Sequence):
        raise InvalidCoefficientError(f"{name} must be an indexed sequence")
    length = len(value)  # type: ignore[arg-type]
    if type(length) is not int or length < 0:
        raise InvalidCoefficientError(
            f"{name} length must be a safe nonnegative integer"
        )
    return length


class CheckedBiquad:
    """Failure-atomic DFII-T section with strict binary64 input validation."""

    __slots__ = ("b0", "b1", "b2", "a1", "a2", "_z1", "_z2")

    def __init__(
        self,
        b0: object,
        b1: object,
        b2: object,
        a0: object,
        a1: object,
        a2: object,
    ) -> None:
        raw = tuple(
            _finite_number(value, f"coefficient[{index}]")
            for index, value in enumerate((b0, b1, b2, a0, a1, a2))
        )
        if raw[3] == 0.0:
            raise InvalidCoefficientError("a0 must be non-zero")

        try:
            normalized = (
                raw[0] / raw[3],
                raw[1] / raw[3],
                raw[2] / raw[3],
                raw[4] / raw[3],
                raw[5] / raw[3],
            )
        except OverflowError as exc:
            raise InvalidCoefficientError(
                "normalized coefficients must be finite"
            ) from exc
        if not all(math.isfinite(value) for value in normalized):
            raise InvalidCoefficientError("normalized coefficients must be finite")

        self.b0, self.b1, self.b2, self.a1, self.a2 = normalized
        self._z1 = 0.0
        self._z2 = 0.0

    @staticmethod
    def _advance(
        b0: float,
        b1: float,
        b2: float,
        a1: float,
        a2: float,
        z1: float,
        z2: float,
        sample: float,
    ) -> tuple[float, float, float]:
        """Calculate output and candidate delays without mutating filter state."""
        output = b0 * sample + z1
        next_z1 = b1 * sample - a1 * output + z2
        next_z2 = b2 * sample - a2 * output
        if not math.isfinite(output):
            raise InvalidCoefficientError("non-finite biquad output")
        if not math.isfinite(next_z1) or not math.isfinite(next_z2):
            raise InvalidCoefficientError("non-finite biquad delay")
        return output, next_z1, next_z2

    def process_sample(self, sample: object) -> float:
        """Process one strictly typed sample, committing state only on success."""
        value = _finite_number(sample, "sample")
        output, next_z1, next_z2 = self._advance(
            self.b0, self.b1, self.b2, self.a1, self.a2,
            self._z1, self._z2, value,
        )
        self._z1, self._z2 = next_z1, next_z2
        return output

    def process(self, signal: _Sequence) -> list[float]:
        """Process a bounded indexed sequence and commit only after full success."""
        length = _length_once(signal, "signal")
        if length == 0:
            raise EmptySignalError("biquad input signal must be non-empty")
        if length > MAX_SAMPLES:
            raise InvalidCoefficientError("signal exceeds MAX_SAMPLES")

        z1, z2 = self._z1, self._z2
        output: list[float] = []
        for index in range(length):
            value = _finite_number(signal[index], f"signal[{index}]")  # type: ignore[index]
            sample_output, z1, z2 = self._advance(
                self.b0, self.b1, self.b2, self.a1, self.a2, z1, z2, value
            )
            output.append(sample_output)

        self._z1, self._z2 = z1, z2
        return output

    def reset(self) -> None:
        """Return the section to positive-zero delay state."""
        self._z1 = 0.0
        self._z2 = 0.0


def checked_sosfilt(sections: _Sequence, signal: _Sequence) -> list[float]:
    """Filter one complete bounded signal through checked binary64 sections."""
    signal_length = _length_once(signal, "signal")
    if signal_length == 0:
        raise EmptySignalError("signal must not be empty")
    if signal_length > MAX_SAMPLES:
        raise InvalidCoefficientError("signal exceeds MAX_SAMPLES")

    section_count = _length_once(sections, "sections")
    if section_count > MAX_ORDER:
        raise InvalidCoefficientError("section count exceeds MAX_ORDER")

    checked_sections: list[CheckedBiquad] = []
    for section_index in range(section_count):
        row = sections[section_index]  # type: ignore[index]
        row_length = _length_once(row, f"section[{section_index}]")
        if row_length != 6:
            raise InvalidCoefficientError("SOS section must have six coefficients")
        coefficients = [row[index] for index in range(6)]  # type: ignore[index]
        checked_sections.append(CheckedBiquad(*coefficients))

    current = [
        _finite_number(signal[index], f"signal[{index}]")  # type: ignore[index]
        for index in range(signal_length)
    ]
    for section in checked_sections:
        current = section.process(current)
    return current
