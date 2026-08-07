from __future__ import annotations

import math
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_dynamic_autograd_labs import (
    DEFAULT_FIXTURE_ROOT,
    DynamicAutogradValidationError,
    execute_case,
    load_json,
    validate_document,
    validate_fixture_root,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-dynamic-graph-and-saved-values.json"


def load_lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_checked_in_corpus_validates() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(load_lab())
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_complete_graph_pins_forward_topology_saved_values_and_backward() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][0], 1e-5)

    assert trace["forward_values"] == {
        "x": 2,
        "w": 3,
        "b": 1,
        "m": 6,
        "z": 7,
        "loss": 49,
    }
    assert trace["topological_order"] == ["x", "w", "m", "b", "z", "loss"]
    assert trace["backward_order"] == ["loss", "z", "b", "m", "w", "x"]
    assert trace["saved_values"]["m"] == [
        {"name": "left", "source_id": "x", "value": 2},
        {"name": "right", "source_id": "w", "value": 3},
    ]
    assert trace["saved_values"]["z"] == []
    assert trace["gradients"] == {
        "loss": 1,
        "z": 14,
        "m": 14,
        "b": 14,
        "x": 42,
        "w": 28,
    }
    assert trace["max_gradient_absolute_error"] < 1e-8


def test_runtime_branch_records_only_the_executed_operation() -> None:
    document = validate_document(load_lab())
    trace = execute_case(document["cases"][1], 1e-5)

    assert trace["branch_choices"] == {"abs_x": "negative"}
    assert trace["executed_operations"]["abs_x"] == "negate"
    assert "identity" not in trace["executed_operations"].values()
    assert trace["forward_values"] == {"x": -2, "abs_x": 2, "loss": 4}
    assert trace["gradients"]["x"] == -4


def test_saved_snapshot_is_not_replaced_by_mutated_live_input() -> None:
    document = validate_document(load_lab())
    case = document["cases"][2]
    mutated = execute_case(case, 1e-5)
    unchanged = execute_case(case, 1e-5, apply_mutations=False)

    assert mutated["forward_values"]["w"] == 3
    assert mutated["live_input_values"]["w"] == 100
    assert mutated["saved_values"]["product"][1]["value"] == 3
    assert mutated["gradients"]["x"] == 3
    assert unchanged["live_input_values"]["w"] == 3
    assert unchanged["gradients"] == mutated["gradients"]


def test_rejects_duplicate_keys_and_non_finite_numbers(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(DynamicAutogradValidationError, match="duplicate JSON key"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text('{"value": NaN}', encoding="utf-8")
    with pytest.raises(DynamicAutogradValidationError, match="non-finite"):
        load_json(non_finite)


def test_rejects_unknown_fields_roster_references_and_mutations() -> None:
    extra = load_lab()
    extra["surprise"] = True
    with pytest.raises(DynamicAutogradValidationError, match="key mismatch"):
        validate_document(extra)

    reordered = load_lab()
    reordered["cases"] = list(reversed(reordered["cases"]))
    with pytest.raises(DynamicAutogradValidationError, match="case ids"):
        validate_document(reordered)

    future_parent = load_lab()
    future_parent["cases"][0]["steps"][0]["inputs"][0] = "z"
    with pytest.raises(DynamicAutogradValidationError, match="must already exist"):
        validate_document(future_parent)

    unknown_mutation = load_lab()
    unknown_mutation["cases"][2]["mutations_after_forward"] = {"ghost": 7}
    with pytest.raises(DynamicAutogradValidationError, match="unknown input"):
        validate_document(unknown_mutation)


def test_rejects_loose_tolerance_dishonest_trace_and_large_numbers() -> None:
    loose = load_lab()
    loose["absolute_tolerance"] = 1e308
    with pytest.raises(DynamicAutogradValidationError, match="canonical"):
        validate_document(loose)

    dishonest = load_lab()
    dishonest["cases"][0]["expected"]["gradients"]["x"] = 999
    with pytest.raises(DynamicAutogradValidationError, match="expected 999"):
        validate_document(dishonest)

    huge_input = load_lab()
    huge_input["cases"][0]["inputs"][0]["value"] = int("9" * 1000)
    with pytest.raises(DynamicAutogradValidationError, match="finite number"):
        validate_document(huge_input)

    huge_expected = load_lab()
    huge_expected["cases"][0]["expected"]["gradients"]["x"] = int("9" * 1000)
    with pytest.raises(DynamicAutogradValidationError, match="finite number"):
        validate_document(huge_expected)

    inaccurate_gradient = load_lab()
    inaccurate_gradient["cases"][0]["inputs"][0]["value"] = 1e6
    with pytest.raises(
        DynamicAutogradValidationError, match="numerical gradient error"
    ):
        validate_document(inaccurate_gradient)


def test_derived_values_and_numerical_gradients_remain_finite() -> None:
    document = validate_document(load_lab())
    for case in document["cases"]:
        trace = execute_case(case, 1e-5)
        assert all(math.isfinite(value) for value in trace["forward_values"].values())
        assert all(math.isfinite(value) for value in trace["gradients"].values())
        assert all(
            math.isfinite(value)
            for value in trace["finite_difference_gradients"].values()
        )


def test_finite_difference_accepts_the_inclusive_input_boundary() -> None:
    case = {
        "inputs": [{"id": "x", "value": 1e6, "requires_gradient": True}],
        "steps": [{"id": "negative", "operation": "negate", "inputs": ["x"]}],
        "output": "negative",
        "mutations_after_forward": {},
    }
    trace = execute_case(case, 1e-5)
    assert trace["forward_values"]["x"] == 1e6
    assert trace["gradients"]["x"] == -1
    assert trace["finite_difference_gradients"]["x"] == pytest.approx(-1, abs=1e-5)
