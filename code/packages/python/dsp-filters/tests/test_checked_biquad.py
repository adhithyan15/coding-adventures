"""Focused public tests for opt-in checked binary64 Biquad/SOS APIs."""

from __future__ import annotations

import math
from collections.abc import Sequence

import pytest

from dsp_filters import (
    MAX_ORDER,
    MAX_SAMPLES,
    CheckedBiquad,
    EmptySignalError,
    InvalidCoefficientError,
    checked_sosfilt,
)


class _PoisonSequence(Sequence[object]):
    def __init__(self, length: int) -> None:
        self.length = length
        self.reads = 0

    def __len__(self) -> int:
        return self.length

    def __getitem__(self, index: int) -> object:
        self.reads += 1
        raise AssertionError(f"unexpected indexed read {index}")


class _RaiseOnSecond(Sequence[object]):
    def __len__(self) -> int:
        return 3

    def __getitem__(self, index: int) -> object:
        if index == 0:
            return 3.0
        if index == 1:
            raise RuntimeError("caller accessor failed")
        return 4.0


class _Zeros(Sequence[object]):
    def __init__(self, length: int) -> None:
        self.length = length

    def __len__(self) -> int:
        return self.length

    def __getitem__(self, index: int) -> object:
        if not 0 <= index < self.length:
            raise IndexError(index)
        return 0.0


def test_checked_direct_division_accepts_minimum_subnormal_identity() -> None:
    minimum_subnormal = float.fromhex("0x0.0000000000001p-1022")
    assert math.isfinite(minimum_subnormal)
    assert minimum_subnormal / minimum_subnormal == 1.0
    identity = CheckedBiquad(
        minimum_subnormal, 0.0, 0.0, minimum_subnormal, 0.0, 0.0
    )
    assert identity.process([1.0]) == [1.0]


def test_checked_inputs_are_strict_and_integer_rounding_is_documented() -> None:
    class FloatSubclass(float):
        pass

    for bad in (True, FloatSubclass(1.0), "1", math.inf, math.nan, 10**10000):
        with pytest.raises(InvalidCoefficientError):
            CheckedBiquad(bad, 0.0, 0.0, 1.0, 0.0, 0.0)

    with pytest.raises(InvalidCoefficientError):
        CheckedBiquad(1.0, 0.0, 0.0, -0.0, 0.0, 0.0)

    identity = CheckedBiquad(1, 0, 0, 1, 0, 0)
    assert identity.process_sample(2**53 + 1) == float(2**53)
    with pytest.raises(InvalidCoefficientError):
        identity.process_sample(True)


def test_checked_sample_rejects_output_and_each_candidate_delay_overflow() -> None:
    large = float.fromhex("0x1.0000000000000p+1023")
    assert math.isfinite(large)

    output_overflow = CheckedBiquad(2.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    with pytest.raises(InvalidCoefficientError):
        output_overflow.process_sample(large)
    assert output_overflow.process_sample(0.0) == 0.0

    z1_overflow = CheckedBiquad(0.0, 2.0, 0.0, 1.0, 0.0, 0.0)
    with pytest.raises(InvalidCoefficientError):
        z1_overflow.process_sample(large)
    assert z1_overflow.process_sample(0.0) == 0.0

    z2_overflow = CheckedBiquad(0.0, 0.0, 2.0, 1.0, 0.0, 0.0)
    with pytest.raises(InvalidCoefficientError):
        z2_overflow.process_sample(large)
    assert z2_overflow.process_sample(0.0) == 0.0

    with pytest.raises(InvalidCoefficientError):
        CheckedBiquad(2.0, 0.0, 0.0, float.fromhex("0x0.0000000000001p-1022"), 0.0, 0.0)


def test_checked_state_is_atomic_for_invalid_samples_accessors_and_overflow() -> None:
    delay = CheckedBiquad(0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
    assert delay.process([1.0, 2.0]) == [0.0, 0.0]
    with pytest.raises(InvalidCoefficientError):
        delay.process([3.0, math.nan, 4.0])
    assert delay.process([0.0, 0.0]) == [1.0, 2.0]

    delay.reset()
    with pytest.raises(RuntimeError, match="caller accessor failed"):
        delay.process(_RaiseOnSecond())
    assert delay.process([0.0, 0.0]) == [0.0, 0.0]

    maximum = float.fromhex("0x1.fffffffffffffp+1023")
    assert math.isfinite(maximum)
    rounded = CheckedBiquad(maximum, 0.0, 0.0, 1.0, 1.0, 0.0)
    assert rounded.process_sample(1.0) == maximum
    with pytest.raises(InvalidCoefficientError):
        rounded.process_sample(2.0)
    rounded.reset()
    assert rounded.process_sample(1.0) == maximum


def test_checked_sos_is_one_shot_ordered_and_preserves_empty_identity_zero() -> None:
    large = float.fromhex("0x1.0000000000000p+1023")
    half = [0.5, 0.0, 0.0, 1.0, 0.0, 0.0]
    gain_two = [2.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    assert checked_sosfilt([half, gain_two], [large]) == [large]
    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt([gain_two, half], [large])

    identity = checked_sosfilt([], [-0.0, 1.0])
    assert math.copysign(1.0, identity[0]) == -1.0
    assert identity[1] == 1.0
    with pytest.raises(EmptySignalError):
        checked_sosfilt([], [])


def test_checked_sos_validation_order_avoids_poison_accessors() -> None:
    empty_sections = _PoisonSequence(1)
    with pytest.raises(EmptySignalError):
        checked_sosfilt(empty_sections, [])
    assert empty_sections.reads == 0

    overlong_signal = _PoisonSequence(MAX_SAMPLES + 1)
    overcap_sections = _PoisonSequence(MAX_ORDER + 1)
    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt(overcap_sections, overlong_signal)
    assert overlong_signal.reads == 0
    assert overcap_sections.reads == 0

    class WrongRow(Sequence[object]):
        def __init__(self, row_length: int) -> None:
            self.row_length = row_length
            self.reads = 0

        def __len__(self) -> int:
            return self.row_length

        def __getitem__(self, index: int) -> object:
            self.reads += 1
            raise AssertionError("coefficient getter ran before row-shape validation")

    for row_length in (5, 7):
        row = WrongRow(row_length)
        with pytest.raises(InvalidCoefficientError):
            checked_sosfilt([row], [1.0])
        assert row.reads == 0


def test_checked_exact_sample_and_section_caps() -> None:
    identity = CheckedBiquad(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    output = identity.process(_Zeros(MAX_SAMPLES))
    assert len(output) == MAX_SAMPLES
    assert output[0] == 0.0 and output[-1] == 0.0

    sections = [([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]) for _ in range(MAX_ORDER)]
    assert checked_sosfilt(sections, [1.0]) == [1.0]
