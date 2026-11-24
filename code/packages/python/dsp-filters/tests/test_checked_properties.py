"""Bounded exact-oracle and raw-binary64 checks for checked filtering."""

from __future__ import annotations

import math
import struct

import pytest
from coding_adventures_test_bootstrap import SplitMix64, for_all, fuzz_bytes, gen_int

from dsp_filters import CheckedBiquad, InvalidCoefficientError


def _direct_oracle(
    coefficients: tuple[int, int, int], samples: list[int]
) -> list[float]:
    expected: list[float] = []
    for index in range(len(samples)):
        direct = sum(
            coefficients[tap] * samples[index - tap]
            for tap in range(3)
            if index >= tap
        )
        expected.append(direct / 128.0)
    return expected


def _check_direct_case(*values: int, fault: bool = False) -> bool:
    coefficients = (values[0], values[1], values[2])
    length = values[3] + 1
    samples = list(values[4 : 4 + length])
    section = CheckedBiquad(
        coefficients[0] / 4.0,
        coefficients[1] / 4.0,
        coefficients[2] / 4.0,
        2.0,
        0.0,
        0.0,
    )
    actual = section.process([sample / 16.0 for sample in samples])
    expected = _direct_oracle(coefficients, samples)
    if fault:
        actual[0] += 1.0
    return actual == expected and all(math.isfinite(value) for value in actual)


def _check_feedback_case(*values: int, fault: bool = False) -> bool:
    coefficients = (values[0], values[1], values[2])
    sign1 = -1 if values[3] == 0 else 1
    sign2 = -1 if values[4] == 0 else 1
    length = values[5] + 1
    samples = list(values[6 : 6 + length])
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

    if fault:
        actual[0] += 1.0
    return actual == expected and all(math.isfinite(value) for value in actual)


_DIRECT_GENS = [gen_int(-4, 4) for _ in range(3)] + [gen_int(0, 31)] + [
    gen_int(-8, 8) for _ in range(32)
]
_FEEDBACK_GENS = [gen_int(-4, 4) for _ in range(3)] + [
    gen_int(0, 1),
    gen_int(0, 1),
    gen_int(0, 11),
] + [gen_int(-8, 8) for _ in range(12)]


def test_ver05_exact_direct_and_feedback_integer_oracles() -> None:
    direct = for_all(
        _check_direct_case,
        *_DIRECT_GENS,
        count=512,
        seed=0xAF02B001,
    )
    assert direct.passed and direct.cases_run == 512
    assert direct.seed == 0xAF02B001

    feedback = for_all(
        _check_feedback_case,
        *_FEEDBACK_GENS,
        count=512,
        seed=0xAF02B002,
    )
    assert feedback.passed and feedback.cases_run == 512
    assert feedback.seed == 0xAF02B002

    broken_direct = for_all(
        lambda *values: _check_direct_case(*values, fault=True),
        *_DIRECT_GENS,
        count=1,
        seed=0xAF02BF05,
    )
    assert not broken_direct.passed
    assert broken_direct.cases_run == 1
    assert broken_direct.shrink_steps > 0
    assert broken_direct.shrunken is not None
    shrunk = broken_direct.shrunken
    assert len(shrunk) == 36
    assert all(-4 <= value <= 4 for value in shrunk[:3])
    assert 0 <= shrunk[3] <= 31
    assert all(-8 <= value <= 8 for value in shrunk[4:])
    assert not _check_direct_case(*shrunk, fault=True)
    assert _check_direct_case(*shrunk)
    broken_direct_replay = for_all(
        lambda *values: _check_direct_case(*values, fault=True),
        *_DIRECT_GENS,
        count=1,
        seed=0xAF02BF05,
    )
    assert broken_direct_replay.counterexample == broken_direct.counterexample
    assert broken_direct_replay.shrunken == broken_direct.shrunken
    assert broken_direct_replay.shrink_steps == broken_direct.shrink_steps

    broken_feedback = for_all(
        lambda *values: _check_feedback_case(*values, fault=True),
        *_FEEDBACK_GENS,
        count=1,
        seed=0xAF02BF06,
    )
    assert not broken_feedback.passed
    assert broken_feedback.cases_run == 1
    assert broken_feedback.shrink_steps > 0
    assert broken_feedback.shrunken is not None
    shrunk_feedback = broken_feedback.shrunken
    assert len(shrunk_feedback) == 18
    assert all(-4 <= value <= 4 for value in shrunk_feedback[:3])
    assert shrunk_feedback[3] in (0, 1)
    assert shrunk_feedback[4] in (0, 1)
    assert 0 <= shrunk_feedback[5] <= 11
    assert all(-8 <= value <= 8 for value in shrunk_feedback[6:])
    assert not _check_feedback_case(*shrunk_feedback, fault=True)
    assert _check_feedback_case(*shrunk_feedback)
    broken_feedback_replay = for_all(
        lambda *values: _check_feedback_case(*values, fault=True),
        *_FEEDBACK_GENS,
        count=1,
        seed=0xAF02BF06,
    )
    assert broken_feedback_replay.counterexample == broken_feedback.counterexample
    assert broken_feedback_replay.shrunken == broken_feedback.shrunken
    assert broken_feedback_replay.shrink_steps == broken_feedback.shrink_steps


def test_seeded_raw_binary64_state_and_error_cases() -> None:
    rng = SplitMix64(0xAF02B003)
    finite = 0
    nonfinite = 0
    for _ in range(128):
        bits = rng.next_u64()
        sample = struct.unpack(">d", bits.to_bytes(8, "big"))[0]
        delay = CheckedBiquad(0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
        if math.isfinite(sample):
            finite += 1
            assert delay.process_sample(sample) == 0.0
            assert delay.process_sample(0.0) == 0.0
            assert delay.process_sample(0.0) == sample
        else:
            nonfinite += 1
            with pytest.raises(InvalidCoefficientError):
                delay.process_sample(sample)
            assert [delay.process_sample(0.0) for _ in range(3)] == [0.0, 0.0, 0.0]
    assert finite + nonfinite == 128
    assert finite == 128 and nonfinite == 0

    boundary_bits = (
        0x0000000000000000,
        0x8000000000000000,
        0x0000000000000001,
        0x0010000000000000,
        0x7FEFFFFFFFFFFFFF,
        0x7FF0000000000000,
        0xFFF0000000000000,
        0x7FF8000000000001,
    )
    boundary_finite = 0
    boundary_nonfinite = 0
    for bits in boundary_bits:
        sample = struct.unpack(">d", bits.to_bytes(8, "big"))[0]
        delay = CheckedBiquad(0.0, 0.0, 1.0, 1.0, 0.0, 0.0)
        if math.isfinite(sample):
            boundary_finite += 1
            assert delay.process_sample(sample) == 0.0
            assert delay.process_sample(0.0) == 0.0
            assert delay.process_sample(0.0) == sample
        else:
            boundary_nonfinite += 1
            with pytest.raises(InvalidCoefficientError):
                delay.process_sample(sample)
            assert [delay.process_sample(0.0) for _ in range(3)] == [0.0, 0.0, 0.0]
    assert (boundary_finite, boundary_nonfinite) == (5, 3)


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
    if sabotage:
        actual += 1.0
    expected = sample
    if actual != expected:
        raise AssertionError(f"checked identity mismatch: {actual!r} != {expected!r}")


def _faulted_identity(payload: bytes) -> bool:
    try:
        _binary64_target(payload, sabotage=True)
    except AssertionError as exc:
        return str(exc).startswith("checked identity mismatch:")
    return False


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
    assert first.corpus_size == replay.corpus_size == 3
    assert first.crashes == replay.crashes == []
    assert first.unique_crashes == replay.unique_crashes == 0
    assert first.is_clean and first == replay

    # The faulty target crashes on every finite binary64, so the seed must not
    # crash: a NaN passes (non-finite inputs are rejected before the fault).
    def faulty_target(payload: bytes) -> None:
        _binary64_target(payload, sabotage=True)

    faulty = fuzz_bytes(
        faulty_target,
        seed=0xAF02BF02,
        iterations=400,
        max_input_bytes=8,
        initial_corpus=[struct.pack(">d", float("nan"))],
    )
    again = fuzz_bytes(
        faulty_target,
        seed=0xAF02BF02,
        iterations=400,
        max_input_bytes=8,
        initial_corpus=[struct.pack(">d", float("nan"))],
    )
    assert faulty == again
    assert faulty.iterations_run == 400 and not faulty.is_clean
    # one record per distinct failure, however many inputs triggered it
    assert faulty.unique_crashes == len(faulty.crashes) >= 1
    record = faulty.crashes[0]
    assert record.kind == "AssertionError" and record.hits >= 1
    # the engine minimises: the all-zero binary64 word is the bytewise lower bound
    reduced = record.minimal_input
    assert reduced == bytes(8)
    assert struct.unpack(">d", reduced)[0] == 0.0
    assert _faulted_identity(reduced)
    _binary64_target(reduced)
