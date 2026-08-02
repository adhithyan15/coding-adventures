from __future__ import annotations

import copy
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_tiny_message_passing_labs import (
    DEFAULT_FIXTURE_ROOT,
    TinyMessagePassingValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-three-node-path.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_schema_and_corpus() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())
    assert validate_corpus() == 1


def test_expands_two_edges_into_four_sorted_messages() -> None:
    messages = execute_lab(lab())["directed_messages"]
    assert [(row["source"], row["target"]) for row in messages] == [
        (1, 0),
        (0, 1),
        (2, 1),
        (1, 2),
    ]
    assert [row["message"] for row in messages] == pytest.approx([1, 0.5, -0.5, 1])


def test_middle_node_sums_two_opposing_messages() -> None:
    middle = execute_lab(lab())["node_updates"][1]
    assert [row["message"] for row in middle["incoming"]] == pytest.approx([0.5, -0.5])
    assert middle["aggregate"] == pytest.approx(0)
    assert middle["self_contribution"] == pytest.approx(0.5)
    assert middle["preactivation"] == pytest.approx(0)
    assert middle["output_feature"] == pytest.approx(0)


def test_shared_update_produces_all_three_outputs() -> None:
    result = validate_document(lab())
    assert [row["aggregate"] for row in result["node_updates"]] == pytest.approx(
        [1, 0, 1]
    )
    assert [row["preactivation"] for row in result["node_updates"]] == pytest.approx(
        [0.75, 0, 0.25]
    )
    assert result["output_features"] == pytest.approx([0.75, 0, 0.25])


def test_rejects_unknown_keys() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(TinyMessagePassingValidationError, match="key mismatch"):
        validate_document(document)


def test_rejects_self_and_duplicate_edges() -> None:
    document = lab()
    document["edges"][0] = {"source": 0, "target": 0}
    with pytest.raises(TinyMessagePassingValidationError, match="non-self"):
        validate_document(document)
    document = lab()
    document["edges"] = [{"source": 0, "target": 1}, {"source": 1, "target": 0}]
    with pytest.raises(TinyMessagePassingValidationError, match="duplicate"):
        validate_document(document)


def test_rejects_invalid_node_indices() -> None:
    document = lab()
    document["edges"][1]["target"] = 9
    with pytest.raises(TinyMessagePassingValidationError, match="invalid"):
        validate_document(document)


def test_rejects_trace_mismatch() -> None:
    document = lab()
    document["expected"]["output_features"][0] = 0.8
    with pytest.raises(TinyMessagePassingValidationError, match="output_features"):
        validate_document(document)


def test_operation_is_exact() -> None:
    document = copy.deepcopy(lab())
    document["operation"]["round"] = "asynchronous"
    with pytest.raises(TinyMessagePassingValidationError, match="operation"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite_json(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"id":"a","id":"b"}', encoding="utf-8")
    with pytest.raises(TinyMessagePassingValidationError, match="duplicate"):
        load_json(duplicate)
    invalid = tmp_path / "invalid.json"
    invalid.write_text('{"value":NaN}', encoding="utf-8")
    with pytest.raises(TinyMessagePassingValidationError, match="non-finite"):
        load_json(invalid)
