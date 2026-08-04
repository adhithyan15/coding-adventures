from __future__ import annotations

import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_training_stabilizer_labs import (
    DEFAULT_FIXTURE_ROOT,
    TrainingStabilizerValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-four-route-comparison.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def trace() -> dict[str, object]:
    return execute_lab(lab())


def route(route_id: str) -> dict[str, object]:
    return next(item for item in trace()["routes"] if item["id"] == route_id)


def test_schema_and_corpus() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())
    assert validate_corpus() == 1


def test_shared_branch_and_normalization_statistics() -> None:
    result = trace()
    assert result["branch"] == [0.5, 0.5, 1.5, 1.5]
    assert result["normalization"] == {
        "mean": 1.0,
        "centered": [-0.5, -0.5, 0.5, 0.5],
        "variance": 0.25,
        "standard_deviation": 0.5,
        "normalized": [-1.0, -1.0, 1.0, 1.0],
        "upstream_sum": 0.0,
        "upstream_dot_normalized": -2.0,
    }


def test_plain_route_is_the_control() -> None:
    plain = route("plain")
    assert plain["output"] == [0.5, 0.5, 1.5, 1.5]
    assert plain["score"] == -1
    assert plain["input_gradient"] == [0.5, 0, 0, -0.5]
    assert plain["weight_gradient"] == -2


def test_normalization_couples_coordinates_and_removes_common_scale() -> None:
    normalized = route("normalization")
    assert normalized["output"] == [-1, -1, 1, 1]
    assert normalized["branch_gradient"] == [1, -1, 1, -1]
    assert normalized["input_gradient"] == [0.5, -0.5, 0.5, -0.5]
    assert normalized["weight_gradient"] == 0


def test_inverted_dropout_pins_training_eval_and_expectation() -> None:
    result = trace()
    assert result["dropout"] == {
        "scaled_mask": [2, 0, 2, 0],
        "evaluation_output": [0.5, 0.5, 1.5, 1.5],
        "training_expectation": [0.5, 0.5, 1.5, 1.5],
    }
    dropout = route("dropout")
    assert dropout["output"] == [1, 0, 3, 0]
    assert dropout["branch_gradient"] == [2, 0, 0, 0]
    assert dropout["input_gradient"] == [1, 0, 0, 0]


def test_residual_route_splits_identity_and_branch_gradients() -> None:
    residual = route("residual")
    assert residual["output"] == [1.5, 1.5, 4.5, 4.5]
    assert residual["skip_gradient"] == [1, 0, 0, -1]
    assert residual["input_gradient"] == [1.5, 0, 0, -1.5]
    assert residual["weight_gradient"] == -2


def test_every_analytical_gradient_passes_finite_difference() -> None:
    for item in validate_document(lab())["routes"]:
        assert max(item["input_gradient_absolute_error"]) < 1e-8
        assert item["weight_gradient_absolute_error"] < 1e-8


def test_rejects_unknown_keys_masks_order_and_trace_drift() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(TrainingStabilizerValidationError, match="key mismatch"):
        validate_document(document)
    document = lab()
    document["dropout_mask"] = [1, 2, 1, 0]
    with pytest.raises(TrainingStabilizerValidationError, match="binary"):
        validate_document(document)
    document = lab()
    document["expected"]["routes"][0]["id"] = "residual"
    with pytest.raises(TrainingStabilizerValidationError, match="route order"):
        validate_document(document)
    document = lab()
    document["expected"]["routes"][3]["input_gradient"][0] = 1.4
    with pytest.raises(TrainingStabilizerValidationError, match="input_gradient"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text('{"x":1,"x":2}', encoding="utf-8")
    with pytest.raises(TrainingStabilizerValidationError, match="duplicate"):
        load_json(path)
    path.write_text('{"x":NaN}', encoding="utf-8")
    with pytest.raises(TrainingStabilizerValidationError, match="non-finite"):
        load_json(path)
