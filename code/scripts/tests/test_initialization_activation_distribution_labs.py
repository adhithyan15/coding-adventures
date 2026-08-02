from __future__ import annotations

import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_initialization_activation_distribution_labs import (  # noqa: E402
    DEFAULT_FIXTURE_ROOT,
    InitializationDistributionValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-three-layer-scale-comparison.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_schema_and_corpus() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())
    assert validate_corpus() == 1


def test_canonical_xavier_tanh_trace() -> None:
    result = execute_lab(lab())["canonical"]
    assert result["layers"][0]["scale"] == pytest.approx(1 / (2**0.5))
    assert result["layers"][0]["activations"][0] == pytest.approx(
        [0.6088593650139138, -0.6088593650139138]
    )
    assert [
        layer["summary"]["standard_deviation"] for layer in result["layers"]
    ] == pytest.approx([0.6088593650139138, 0.49271338636057294, 0.4563673571184874])


def test_tiny_relu_shrinks_across_layers() -> None:
    row = execute_lab(lab())["comparison"][1]
    assert row["initializer"] == "tiny"
    assert row["activation"] == "relu"
    assert row["standard_deviations"] == pytest.approx(
        [0.05, 0.006959705453537528, 0.0008569568250501307]
    )


def test_large_modes_expose_saturation_and_growth() -> None:
    comparison = execute_lab(lab())["comparison"]
    large_tanh, large_relu = comparison[6], comparison[7]
    assert large_tanh["saturated_fractions"] == [1, 0.5, 1]
    assert large_relu["standard_deviations"] == pytest.approx(
        [1, 2.7838821814150108, 6.855654600401044]
    )


def test_relu_zero_fraction_is_traced() -> None:
    comparison = execute_lab(lab())["comparison"]
    for row in comparison:
        if row["activation"] == "relu":
            assert row["zero_fractions"] == [0.5, 0.5, 0.625]
            assert row["saturated_fractions"] == [0, 0, 0]


def test_rejects_unknown_keys_and_trace_mismatch() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(InitializationDistributionValidationError, match="key mismatch"):
        validate_document(document)
    document = lab()
    document["expected"]["comparison"][0]["standard_deviations"][0] = 9
    with pytest.raises(
        InitializationDistributionValidationError, match="standard_deviations"
    ):
        validate_document(document)


def test_rejects_bad_shape_and_operation() -> None:
    document = lab()
    document["inputs"][1] = [0, 1, 2]
    with pytest.raises(InitializationDistributionValidationError, match="rectangular"):
        validate_document(document)
    document = lab()
    document["weight_templates"][1] = [[1, -1]]
    with pytest.raises(InitializationDistributionValidationError, match="fan-in"):
        validate_document(document)
    document = lab()
    document["operation"]["variance"] = "sample"
    with pytest.raises(InitializationDistributionValidationError, match="operation"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text('{"x":1,"x":2}', encoding="utf-8")
    with pytest.raises(InitializationDistributionValidationError, match="duplicate"):
        load_json(path)
    path.write_text('{"x":Infinity}', encoding="utf-8")
    with pytest.raises(InitializationDistributionValidationError, match="non-finite"):
        load_json(path)
