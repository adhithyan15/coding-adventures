"""Shared AF-02 fixture checks through the public package exports."""

from __future__ import annotations

import json
import math
from pathlib import Path
from typing import Any

import pytest

from dsp_filters import Biquad, EmptySignalError, sosfilt

FIXTURE = (
    Path(__file__).resolve().parents[4]
    / "specs"
    / "fixtures"
    / "biquad-sos-v1"
    / "cases.json"
)
RECORD_KEYS = {
    "biquad.process": {"id", "operation", "sections", "expected", "chunk_lengths"},
    "sosfilt": {"id", "operation", "sections", "expected"},
}
EXPECTED_RECORDS = {
    "biquad.process.impulse16": "biquad.process",
    "sosfilt.impulse16.two_sections": "sosfilt",
}


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field {key!r}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant {value!r}")


def _finite_number(value: Any, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{where} must be a JSON number")
    try:
        number = float(value)
    except OverflowError as exc:
        raise ValueError(f"{where} is outside finite binary64 range") from exc
    if not math.isfinite(number):
        raise ValueError(f"{where} must be finite")
    return number


def _exact_keys(value: Any, keys: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise ValueError(f"{where} must contain exactly fields {sorted(keys)}")
    return value


def _decode_fixture(raw: str) -> tuple[list[float], list[dict[str, Any]]]:
    document = json.loads(
        raw,
        object_pairs_hook=_unique_object,
        parse_constant=_reject_constant,
    )
    root = _exact_keys(
        document,
        {"schema_version", "input", "comparison", "cases"},
        "fixture",
    )
    if root["schema_version"] != 1 or isinstance(root["schema_version"], bool):
        raise ValueError("schema_version must be integer 1")
    if root["comparison"] != "finite_exact_numeric_equal_zero_sign_ignored":
        raise ValueError("unsupported comparison rule")
    signal = root["input"]
    if not isinstance(signal, list) or len(signal) != 16:
        raise ValueError("input must contain exactly 16 samples")
    samples = [_finite_number(value, f"input[{i}]") for i, value in enumerate(signal)]
    if samples != [1.0, *([0.0] * 15)]:
        raise ValueError("input must be the 16-sample unit impulse")
    records = root["cases"]
    if not isinstance(records, list) or len(records) != 2:
        raise ValueError("fixture must contain exactly two operation records")

    by_id: dict[str, dict[str, Any]] = {}
    for index, raw_record in enumerate(records):
        if not isinstance(raw_record, dict):
            raise ValueError(f"cases[{index}] must be an object")
        operation = raw_record.get("operation")
        if operation not in RECORD_KEYS:
            raise ValueError(f"cases[{index}] has an unknown operation")
        record = _exact_keys(raw_record, RECORD_KEYS[operation], f"cases[{index}]")
        record_id = record["id"]
        if not isinstance(record_id, str) or record_id in by_id:
            raise ValueError("operation IDs must be unique strings")
        if EXPECTED_RECORDS.get(record_id) != operation:
            raise ValueError(f"unexpected ID/operation pair {record_id!r}")
        sections = record["sections"]
        expected_sections = 1 if operation == "biquad.process" else 2
        if not isinstance(sections, list) or len(sections) != expected_sections:
            raise ValueError(f"{record_id}: wrong section count")
        for section_index, section in enumerate(sections):
            if not isinstance(section, list) or len(section) != 6:
                raise ValueError(f"{record_id}: section must have six coefficients")
            sections[section_index] = [
                _finite_number(value, f"sections[{section_index}][{coefficient_index}]")
                for coefficient_index, value in enumerate(section)
            ]
        record["sections"] = sections
        expected = record["expected"]
        if not isinstance(expected, list) or len(expected) != 16:
            raise ValueError(f"{record_id}: expected must contain 16 samples")
        record["expected"] = [
            _finite_number(value, f"expected[{i}]") for i, value in enumerate(expected)
        ]
        if operation == "biquad.process":
            chunks = record["chunk_lengths"]
            if (
                not isinstance(chunks, list)
                or any(type(length) is not int or length <= 0 for length in chunks)
                or sum(chunks) != 16
            ):
                raise ValueError(
                    "chunk_lengths must be positive integers summing to 16"
                )
            if chunks != [3, 1, 12]:
                raise ValueError("chunk_lengths must equal [3, 1, 12]")
        by_id[record_id] = record
    if set(by_id) != set(EXPECTED_RECORDS):
        raise ValueError("fixture is missing a required operation ID")
    return samples, [by_id[key] for key in EXPECTED_RECORDS]


def _assert_exact_finite(actual: list[float], expected: list[float]) -> None:
    assert len(actual) == len(expected)
    for index, (got, want) in enumerate(zip(actual, expected, strict=True)):
        assert math.isfinite(got), f"output[{index}] is non-finite: {got}"
        assert got == want, f"output[{index}] value mismatch: {got!r} != {want!r}"


def _biquad(record: dict[str, Any]) -> Biquad:
    return Biquad(*record["sections"][0])


def _partition_chunks(signal: list[float], mask: int) -> list[list[float]]:
    chunks: list[list[float]] = []
    start = 0
    for boundary in range(15):
        if mask & (1 << boundary):
            chunks.append(signal[start : boundary + 1])
            start = boundary + 1
    chunks.append(signal[start:])
    return chunks


def _process_chunks(filter_: Biquad, chunks: list[list[float]]) -> list[float]:
    output: list[float] = []
    for chunk in chunks:
        output.extend(filter_.process(chunk))
    return output


def _next_u32(state: list[int]) -> int:
    value = state[0]
    value ^= (value << 13) & 0xFFFFFFFF
    value ^= value >> 17
    value ^= (value << 5) & 0xFFFFFFFF
    state[0] = value & 0xFFFFFFFF
    return state[0]


def test_shared_fixture_all_biquad_partitions_and_state_semantics() -> None:
    signal, records = _decode_fixture(FIXTURE.read_text(encoding="utf-8"))
    record = records[0]
    expected = record["expected"]
    assert all(
        type(coefficient) is float
        for section in record["sections"]
        for coefficient in section
    )

    _assert_exact_finite(_biquad(record).process(signal), expected)
    chunked = _biquad(record)
    _assert_exact_finite(
        _process_chunks(chunked, [signal[:3], signal[3:4], signal[4:]]), expected
    )
    samplewise = _biquad(record)
    _assert_exact_finite([samplewise.process_sample(x) for x in signal], expected)

    # The same object carries state across every chunk in each partition.
    for mask in range(1 << 15):
        filter_ = _biquad(record)
        _assert_exact_finite(
            _process_chunks(filter_, _partition_chunks(signal, mask)), expected
        )

    replay = _biquad(record)
    replay.process(signal[:7])
    replay.reset()
    _assert_exact_finite(replay.process(signal), expected)


def test_shared_fixture_sosfilt_is_one_shot_and_empty_behavior() -> None:
    signal, records = _decode_fixture(FIXTURE.read_text(encoding="utf-8"))
    record = records[1]
    sections = record["sections"]
    _assert_exact_finite(sosfilt(sections, signal), record["expected"])
    _assert_exact_finite(sosfilt(sections, signal), record["expected"])
    _assert_exact_finite(sosfilt([], signal), signal)
    with pytest.raises(EmptySignalError):
        sosfilt([], [])


def test_fixture_comparison_treats_signed_zero_as_equal() -> None:
    _assert_exact_finite([-0.0], [0.0])
    _assert_exact_finite([0.0], [-0.0])


def test_seeded_partition_property_samples() -> None:
    """Bounded property samples: 64 vectors, each checked under random splits."""
    _, records = _decode_fixture(FIXTURE.read_text(encoding="utf-8"))
    record = records[0]
    state = [0xAF02A11]
    for _ in range(64):
        length = 1 + _next_u32(state) % 32
        signal = [((_next_u32(state) & 0xFFFF) - 32768) / 32768 for _ in range(length)]
        boundaries = [
            index
            for index in range(1, length)
            if _next_u32(state) & 1
        ]
        chunks: list[list[float]] = []
        start = 0
        for end in boundaries:
            chunks.append(signal[start:end])
            start = end
        chunks.append(signal[start:])
        expected = _biquad(record).process(signal)
        actual = _process_chunks(_biquad(record), chunks)
        _assert_exact_finite(actual, expected)


def test_seeded_fixture_schema_fuzz_cases() -> None:
    """Deterministic malformed-fixture fuzz sample: 128 mutations, seed 0xF102."""
    raw = FIXTURE.read_text(encoding="utf-8")
    state = [0xF102]
    for _ in range(128):
        document = json.loads(raw)
        mutation = _next_u32(state) % 12
        if mutation == 0:
            document["schema_version"] = True
        elif mutation == 1:
            del document["comparison"]
        elif mutation == 2:
            document["unknown"] = 0
        elif mutation == 3:
            document["cases"].pop()
        elif mutation == 4:
            document["cases"][1]["id"] = document["cases"][0]["id"]
        elif mutation == 5:
            document["cases"][0]["unexpected"] = 0
        elif mutation == 6:
            document["cases"][0]["sections"][0][0] = True
        elif mutation == 7:
            document["input"][0] = "1"
        elif mutation == 8:
            document["cases"][0]["expected"].pop()
        elif mutation == 9:
            document["cases"][0]["chunk_lengths"] = [16]
        elif mutation == 10:
            document["cases"][0]["expected"][0] = float("nan")
        else:
            document["cases"][0]["id"] = "unknown.operation"
        with pytest.raises(ValueError):
            _decode_fixture(json.dumps(document))


def test_shared_fixture_loader_rejects_malformed_and_mutated_data() -> None:
    raw = FIXTURE.read_text(encoding="utf-8")
    parsed = json.loads(raw)

    missing = json.loads(raw)
    missing["cases"].pop()
    with pytest.raises(ValueError, match="exactly two"):
        _decode_fixture(json.dumps(missing))

    for invalid_version in (True, 2):
        invalid = json.loads(raw)
        invalid["schema_version"] = invalid_version
        with pytest.raises(ValueError, match="schema_version"):
            _decode_fixture(json.dumps(invalid))

    unknown_field = json.loads(raw)
    unknown_field["extra"] = 1
    with pytest.raises(ValueError, match="exactly fields"):
        _decode_fixture(json.dumps(unknown_field))

    missing_field = json.loads(raw)
    del missing_field["comparison"]
    with pytest.raises(ValueError, match="exactly fields"):
        _decode_fixture(json.dumps(missing_field))

    duplicate = json.loads(raw)
    duplicate["cases"][1]["id"] = duplicate["cases"][0]["id"]
    with pytest.raises(ValueError, match="unique"):
        _decode_fixture(json.dumps(duplicate))

    extra_record_field = json.loads(raw)
    extra_record_field["cases"][0]["extra"] = 0
    with pytest.raises(ValueError, match="exactly fields"):
        _decode_fixture(json.dumps(extra_record_field))

    malformed = json.loads(raw)
    malformed["cases"][0]["sections"][0][0] = True
    with pytest.raises(ValueError, match="JSON number"):
        _decode_fixture(json.dumps(malformed))

    short = json.loads(raw)
    short["cases"][0]["sections"][0].pop()
    with pytest.raises(ValueError, match="six coefficients"):
        _decode_fixture(json.dumps(short))

    wrong_chunks = json.loads(raw)
    wrong_chunks["cases"][0]["chunk_lengths"] = [16]
    with pytest.raises(ValueError, match=r"equal \[3, 1, 12\]"):
        _decode_fixture(json.dumps(wrong_chunks))

    changed_expected = json.loads(raw)
    changed_expected["cases"][0]["expected"][3] += 2**-20
    _, changed_records = _decode_fixture(json.dumps(changed_expected))
    with pytest.raises(AssertionError, match="value mismatch"):
        _assert_exact_finite(
            _biquad(changed_records[0]).process(parsed["input"]),
            changed_records[0]["expected"],
        )

    with pytest.raises(ValueError, match="non-standard"):
        _decode_fixture(raw.replace("0.25,", "NaN,", 1))
    with pytest.raises(ValueError, match="duplicate JSON field"):
        _decode_fixture(
            raw.replace(
                '"schema_version": 1,',
                '"schema_version": 1, "schema_version": 1,',
                1,
            )
        )
    with pytest.raises(ValueError, match="finite"):
        _decode_fixture(raw.replace("0.25,", "1e999,", 1))
    with pytest.raises(ValueError, match="binary64 range"):
        _decode_fixture(raw.replace("0.25,", f"{10**400},", 1))
