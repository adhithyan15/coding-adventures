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


class _RaiseOnLength(Sequence[object]):
    def __len__(self) -> int:
        raise RuntimeError("caller length failed")

    def __getitem__(self, index: int) -> object:
        raise AssertionError(f"unexpected indexed read {index}")


class _IndexHook:
    def __index__(self) -> int:
        raise AssertionError("length validation must not call __index__")


class _RawLengthSequence(Sequence[object]):
    def __init__(self, raw_length: object) -> None:
        self.raw_length = raw_length

    def __len__(self) -> int:  # type: ignore[override]
        return self.raw_length  # type: ignore[return-value]

    def __getitem__(self, index: int) -> object:
        raise AssertionError(f"unexpected indexed read {index}")


class _StaticLengthSequence(Sequence[object]):
    @staticmethod
    def __len__() -> int:
        return 1

    def __getitem__(self, index: int) -> object:
        if index == 0:
            return 1.0
        raise IndexError(index)


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
    assert rounded.process_sample(0.0) == -maximum
    rounded.reset()
    assert rounded.process_sample(1.0) == maximum


def test_checked_lengths_are_captured_once_and_caller_length_errors_propagate() -> None:
    class CountedSequence(Sequence[object]):
        def __init__(self) -> None:
            self.length_reads = 0

        def __len__(self) -> int:
            self.length_reads += 1
            return 1

        def __getitem__(self, index: int) -> object:
            if index == 0:
                return 1.0
            raise IndexError(index)

    signal = CountedSequence()
    assert CheckedBiquad(1, 0, 0, 1, 0, 0).process(signal) == [1.0]
    assert signal.length_reads == 1
    with pytest.raises(RuntimeError, match="caller length failed"):
        CheckedBiquad(1, 0, 0, 1, 0, 0).process(_RaiseOnLength())

    assert CheckedBiquad(1, 0, 0, 1, 0, 0).process(_StaticLengthSequence()) == [1.0]
    for raw_length in (True, 1.0, _IndexHook(), -1, 2**100):
        with pytest.raises(InvalidCoefficientError):
            CheckedBiquad(1, 0, 0, 1, 0, 0).process(_RawLengthSequence(raw_length))


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
    class PoisonSectionLength(Sequence[object]):
        def __len__(self) -> int:
            raise AssertionError("section length getter ran before signal check")

        def __getitem__(self, index: int) -> object:
            raise AssertionError("section indexed before signal check")

    empty_sections = PoisonSectionLength()
    with pytest.raises(EmptySignalError):
        checked_sosfilt(empty_sections, [])

    overlong_signal = _PoisonSequence(MAX_SAMPLES + 1)
    overcap_sections = _PoisonSequence(MAX_ORDER + 1)
    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt(overcap_sections, overlong_signal)
    assert overlong_signal.reads == 0
    assert overcap_sections.reads == 0

    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt(overcap_sections, [1.0])
    assert overcap_sections.reads == 0

    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt([object()], [1.0])

    with pytest.raises(InvalidCoefficientError):
        CheckedBiquad(1, 0, 0, 1, 0, 0).process(_PoisonSequence(MAX_SAMPLES + 1))

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

    class InvalidFirstSlot(Sequence[object]):
        def __init__(self) -> None:
            self.reads: list[int] = []

        def __len__(self) -> int:
            return 6

        def __getitem__(self, index: int) -> object:
            self.reads.append(index)
            if index == 0:
                return True
            if index == 1:
                raise AssertionError("later slot read after invalid first value")
            return 0.0

    row = InvalidFirstSlot()
    with pytest.raises(InvalidCoefficientError):
        checked_sosfilt([row], [1.0])
    assert row.reads == [0]


def test_normalized_coefficients_are_not_public_writable_attributes() -> None:
    filter_ = CheckedBiquad(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    assert not hasattr(filter_, "b0")
    assert not hasattr(filter_, "a1")


def test_checked_exact_sample_and_section_caps() -> None:
    identity = CheckedBiquad(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    output = identity.process(_Zeros(MAX_SAMPLES))
    assert len(output) == MAX_SAMPLES
    assert output[0] == 0.0 and output[-1] == 0.0

    sections = [([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]) for _ in range(MAX_ORDER)]
    assert checked_sosfilt(sections, [1.0]) == [1.0]
