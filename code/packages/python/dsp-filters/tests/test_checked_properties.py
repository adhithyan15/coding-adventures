"""Bounded exact-oracle and raw-binary64 checks for checked filtering."""

from __future__ import annotations

import math
import struct

import pytest
from coding_adventures_test_bootstrap import Gen, SplitMix64, for_all, fuzz_bytes

from dsp_filters import CheckedBiquad, InvalidCoefficientError


def _fir_case(seed: int) -> tuple[list[int], list[int]]:
    rng = SplitMix64(seed)
    coefficients = [rng.bounded_range(-4, 4) for _ in range(3)]
    samples = [rng.bounded_range(-8, 8) for _ in range(rng.bounded_range(1, 32))]
    return coefficients, samples


def _feedback_case(seed: int) -> tuple[list[int], int, int, list[int]]:
    rng = SplitMix64(seed)
    coefficients = [rng.bounded_range(-4, 4) for _ in range(3)]
    sign1 = -1 if rng.bounded(2) == 0 else 1
    sign2 = -1 if rng.bounded(2) == 0 else 1
    samples = [rng.bounded_range(-8, 8) for _ in range(rng.bounded_range(1, 12))]
    return coefficients, sign1, sign2, samples


def _direct_oracle(coefficients: list[int], samples: list[int]) -> list[float]:
    expected: list[float] = []
    for index in range(len(samples)):
        direct = sum(
            coefficients[tap] * samples[index - tap]
            for tap in range(3)
            if index >= tap
        )
        expected.append(direct / 128.0)
    return expected


def _check_direct_seed(seed: int) -> bool:
    coefficients, samples = _fir_case(seed)
    section = CheckedBiquad(
        coefficients[0] / 4.0,
        coefficients[1] / 4.0,
        coefficients[2] / 4.0,
        2.0,
        0.0,
        0.0,
    )
    actual = section.process([sample / 16.0 for sample in samples])
    return actual == _direct_oracle(coefficients, samples) and all(
        math.isfinite(value) for value in actual
    )


def _check_feedback_seed(seed: int) -> bool:
    coefficients, sign1, sign2, samples = _feedback_case(seed)
    section = CheckedBiquad(
        coefficients[0] / 4.0,
        coefficients[1] / 4.0,
        coefficients[2] / 4.0,
        2.0,
        float(sign1),
        sign2 / 2.0,
    )
    actual = section.process([sample / 16.0 for sample in samples])

    y_prev1 = 0
    y_prev2 = 0
    expected: list[float] = []
    for index in range(len(samples)):
        direct = sum(
            coefficients[tap] * samples[index - tap]
            for tap in range(3)
            if index >= tap
        )
        scaled = (1 << index) * direct - sign1 * y_prev1 - sign2 * y_prev2
        expected.append(scaled / float(1 << (7 + index)))
        y_prev2, y_prev1 = y_prev1, scaled

    return actual == expected and all(math.isfinite(value) for value in actual)


def test_ver05_exact_direct_and_feedback_integer_oracles() -> None:
    direct = for_all(
        _check_direct_seed,
        Gen(lambda rng: rng.next_u64()),
        count=512,
        seed=0xAF02B001,
    )
    assert direct.passed and direct.cases_run == 512
    assert direct.seed == 0xAF02B001

    feedback = for_all(
        _check_feedback_seed,
        Gen(lambda rng: rng.next_u64()),
        count=512,
        seed=0xAF02B002,
    )
    assert feedback.passed and feedback.cases_run == 512
    assert feedback.seed == 0xAF02B002


def test_seeded_raw_binary64_state_and_error_cases() -> None:
    rng = SplitMix64(0xAF02B003)
    for _ in range(128):
        bits = rng.next_u64()
        sample = struct.unpack(">d", bits.to_bytes(8, "big"))[0]
        delay = CheckedBiquad(0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
        if math.isfinite(sample):
            assert delay.process_sample(sample) == 0.0
            assert delay.process_sample(0.0) == 0.0
            assert delay.process_sample(0.0) == sample
        else:
            with pytest.raises(InvalidCoefficientError):
                delay.process_sample(sample)
            assert [delay.process_sample(0.0) for _ in range(3)] == [0.0, 0.0, 0.0]


def _binary64_target(payload: bytes, *, sabotage: bool = False) -> None:
    """The Python fuzz callback raises on a semantic violation; returns are ignored."""
    if len(payload) != 8:
        return
    sample = struct.unpack(">d", payload)[0]
    filter_ = CheckedBiquad(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    if not math.isfinite(sample):
        with pytest.raises(InvalidCoefficientError):
            filter_.process_sample(sample)
        return
    actual = filter_.process_sample(sample)
    expected = sample + 1.0 if sabotage else sample
    if actual != expected:
        raise AssertionError(f"checked identity mismatch: {actual!r} != {expected!r}")


def test_ver06_seeded_mutation_fuzz_replays_and_detects_same_target_fault() -> None:
    witness = struct.pack(">d", 1.0)
    _binary64_target(witness)
    with pytest.raises(AssertionError, match="identity mismatch"):
        _binary64_target(witness, sabotage=True)

    initial = [witness, struct.pack(">d", 0.0), struct.pack(">d", float("nan"))]
    first = fuzz_bytes(
        _binary64_target,
        seed=0xAF02BF01,
        iterations=256,
        max_input_bytes=8,
        initial_corpus=initial,
    )
    replay = fuzz_bytes(
        _binary64_target,
        seed=0xAF02BF01,
        iterations=256,
        max_input_bytes=8,
        initial_corpus=initial,
    )
    assert first.iterations_run == replay.iterations_run == 256
    assert first.seed == replay.seed == 0xAF02BF01
    assert first.crashes == replay.crashes == []
    assert first.unique_crashes == replay.unique_crashes == 0

    faulty = fuzz_bytes(
        lambda payload: _binary64_target(payload, sabotage=True),
        seed=0xAF02BF02,
        iterations=64,
        max_input_bytes=8,
        initial_corpus=[witness],
    )
    assert faulty.iterations_run == 64
    assert faulty.crashes
    failing_input = faulty.crashes[0].input_data
    assert isinstance(failing_input, bytes)
    with pytest.raises(AssertionError, match="identity mismatch"):
        _binary64_target(failing_input, sabotage=True)
    _binary64_target(failing_input)

