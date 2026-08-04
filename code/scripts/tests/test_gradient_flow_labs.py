from __future__ import annotations

import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_gradient_flow_labs import (
    DEFAULT_FIXTURE_ROOT,
    GradientFlowValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-four-layer-gradient-flow.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def traces() -> list[dict[str, object]]:
    return execute_lab(lab())["traces"]


def test_schema_and_corpus() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())
    assert validate_corpus() == 1


def test_small_tanh_gradient_vanishes() -> None:
    trace = traces()[0]
    assert trace["classification"] == "vanishing"
    assert trace["chain_jacobian"] == pytest.approx(0.045877150455727246)
    assert trace["input_gradient"] == pytest.approx(0.0025900181205328957)


def test_saturated_tanh_derivatives_overwhelm_large_weights() -> None:
    trace = traces()[1]
    assert [layer["weight"] for layer in trace["layers"]] == [3, 3, 3, 3]
    assert trace["layers"][0]["activation_derivative"] == pytest.approx(
        0.009866037165440211
    )
    assert trace["chain_jacobian"] == pytest.approx(8.400447769691746e-7)


def test_unit_relu_is_stable() -> None:
    trace = traces()[2]
    assert trace["classification"] == "stable"
    assert trace["chain_jacobian"] == 1
    assert trace["input_gradient"] == 1


def test_large_relu_explodes_exactly() -> None:
    trace = traces()[3]
    assert trace["classification"] == "exploding"
    assert [layer["activation"] for layer in trace["layers"]] == [2, 4, 8, 16]
    assert [layer["weight_gradient"] for layer in trace["layers"]] == [
        128,
        128,
        128,
        128,
    ]
    assert trace["chain_jacobian"] == 16
    assert trace["input_gradient"] == 256


def test_all_input_gradients_pass_finite_difference() -> None:
    for trace in validate_document(lab())["traces"]:
        assert trace["finite_difference_error"] < 1e-8


def test_rejects_unknown_keys_scenario_order_and_trace_drift() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(GradientFlowValidationError, match="key mismatch"):
        validate_document(document)
    document = lab()
    document["scenarios"][0]["id"] = "wrong"
    with pytest.raises(GradientFlowValidationError, match="scenario order"):
        validate_document(document)
    document = lab()
    document["expected"]["traces"][3]["input_gradient"] = 255
    with pytest.raises(GradientFlowValidationError, match="input_gradient"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text('{"x":1,"x":2}', encoding="utf-8")
    with pytest.raises(GradientFlowValidationError, match="duplicate"):
        load_json(path)
    path.write_text('{"x":NaN}', encoding="utf-8")
    with pytest.raises(GradientFlowValidationError, match="non-finite"):
        load_json(path)
