from __future__ import annotations

import math
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_gradient_accumulation_labs import (
    DEFAULT_FIXTURE_ROOT,
    GradientAccumulationValidationError,
    execute_case,
    load_json,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-gradient-buffer-schedules.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_validates() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_two_backward_calls_add_into_one_persistent_buffer() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][0], 1e-5)

    assert [step["local_gradient"] for step in trace["steps"]] == [2, 2]
    assert [
        (step["buffer_before"], step["buffer_after"]) for step in trace["steps"]
    ] == [
        (0, 2),
        (2, 4),
    ]
    assert trace["final_parameter"] == 1
    assert trace["final_gradient_buffer"] == 4
    assert trace["max_gradient_absolute_error"] < 1e-8


def test_zero_between_calls_makes_the_second_gradient_stand_alone() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][1], 1e-5)

    assert [step["kind"] for step in trace["steps"]] == [
        "backward",
        "zero_grad",
        "backward",
    ]
    assert trace["steps"][1] == {
        "index": 1,
        "kind": "zero_grad",
        "parameter_before": 1,
        "parameter_after": 1,
        "buffer_before": 2,
        "buffer_after": 0,
    }
    assert trace["final_gradient_buffer"] == 2


def test_mean_update_reads_the_buffer_and_explicit_zero_clears_it() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][2], 1e-5)
    optimizer = trace["steps"][2]
    zero = trace["steps"][3]

    assert optimizer["buffer_before"] == optimizer["buffer_after"] == 4
    assert optimizer["applied_gradient"] == 2
    assert optimizer["parameter_delta"] == pytest.approx(-0.2)
    assert optimizer["parameter_after"] == pytest.approx(0.8)
    assert zero["buffer_before"] == 4
    assert zero["buffer_after"] == 0
    assert trace["final_gradient_buffer"] == 0


def test_forgotten_zero_contaminates_the_next_optimizer_step() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][3], 1e-5)

    next_backward = trace["steps"][3]
    wrong_step = trace["steps"][4]
    assert next_backward["local_gradient"] == pytest.approx(0.8)
    assert next_backward["buffer_before"] == 4
    assert next_backward["buffer_after"] == pytest.approx(4.8)
    assert wrong_step["applied_gradient"] == pytest.approx(4.8)
    assert trace["final_parameter"] == pytest.approx(0.32)


def test_rejects_duplicate_keys_and_non_finite_numbers(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(GradientAccumulationValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": Infinity}', encoding="utf-8")
    with pytest.raises(GradientAccumulationValidationError, match="non-finite"):
        load_json(non_finite)


def test_rejects_unknown_fields_roster_samples_and_divisors() -> None:
    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(GradientAccumulationValidationError, match="key mismatch"):
        validate_document(extra)

    reordered = load_lab()
    reordered["cases"] = list(reversed(reordered["cases"]))
    with pytest.raises(GradientAccumulationValidationError, match="case ids"):
        validate_document(reordered)

    unknown_sample = load_lab()
    unknown_sample["cases"][0]["events"][0]["sample_id"] = "ghost"
    with pytest.raises(GradientAccumulationValidationError, match="unknown sample"):
        validate_document(unknown_sample)

    bad_divisor = load_lab()
    bad_divisor["cases"][2]["events"][2]["divisor"] = 0
    with pytest.raises(GradientAccumulationValidationError, match="divisor"):
        validate_document(bad_divisor)


def test_rejects_loose_tolerance_dishonest_trace_and_large_numbers() -> None:
    loose = load_lab()
    loose["absolute_tolerance"] = 1
    with pytest.raises(GradientAccumulationValidationError, match="canonical"):
        validate_document(loose)

    dishonest = load_lab()
    dishonest["cases"][0]["expected"]["final_gradient_buffer"] = 999
    with pytest.raises(GradientAccumulationValidationError, match="expected 999"):
        validate_document(dishonest)

    huge = load_lab()
    huge["cases"][0]["initial_parameter"] = int("9" * 1000)
    with pytest.raises(GradientAccumulationValidationError, match="finite number"):
        validate_document(huge)

    inaccurate = load_lab()
    inaccurate["cases"][0]["samples"][0]["input"] = 1000
    with pytest.raises(
        GradientAccumulationValidationError, match="numerical gradient error"
    ):
        validate_document(inaccurate)


def test_every_derived_trace_value_is_finite() -> None:
    document = validate_document(load_lab())
    for case in document["cases"]:
        trace = execute_case(case, 1e-5)
        assert math.isfinite(trace["final_parameter"])
        assert math.isfinite(trace["final_gradient_buffer"])
        for step in trace["steps"]:
            assert all(
                math.isfinite(value)
                for value in step.values()
                if isinstance(value, float)
            )
