from __future__ import annotations

import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "code" / "scripts"))

from validate_graph_convolution_attention_labs import (  # noqa: E402
    DEFAULT_FIXTURE_ROOT,
    GraphConvolutionAttentionValidationError,
    execute_lab,
    load_json,
    validate_corpus,
    validate_document,
)

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-three-node-gcn-gat.json"


def lab() -> dict[str, object]:
    return load_json(LAB_PATH)


def test_schema_and_corpus() -> None:
    schema = load_json(DEFAULT_FIXTURE_ROOT / "schema.json")
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(lab())
    assert validate_corpus() == 1


def test_degrees_and_gcn_outputs() -> None:
    result = execute_lab(lab())
    assert result["degrees"] == [2, 3, 2]
    assert result["gcn_outputs"] == pytest.approx(
        [1.3164965809277263, 2 / 3, 0.31649658092772615]
    )


def test_middle_gcn_exposes_all_coefficients() -> None:
    row = execute_lab(lab())["gcn"][1]
    assert [item["coefficient"] for item in row["rows"]] == pytest.approx(
        [1 / (6**0.5), 1 / 3, 1 / (6**0.5)]
    )
    assert [item["contribution"] for item in row["rows"]] == pytest.approx(
        [0.4082482904638631, 2 / 3, -0.4082482904638631]
    )


def test_middle_attention_is_stable_and_normalized() -> None:
    row = execute_lab(lab())["gat"][1]
    assert row["maximum_score"] == 2
    assert [item["shifted_score"] for item in row["rows"]] == [-1, 0, -3]
    assert sum(item["attention_weight"] for item in row["rows"]) == pytest.approx(1)
    assert row["output"] == pytest.approx(1.6351464587795619)


def test_all_gat_outputs() -> None:
    assert validate_document(lab())["gat_outputs"] == pytest.approx(
        [1.7310585786300048, 1.6351464587795619, 1.8577223804673]
    )


def test_rejects_unknown_keys_and_trace_mismatch() -> None:
    document = lab()
    document["surprise"] = True
    with pytest.raises(GraphConvolutionAttentionValidationError, match="key mismatch"):
        validate_document(document)
    document = lab()
    document["expected"]["gat_outputs"][0] = 9
    with pytest.raises(GraphConvolutionAttentionValidationError, match="gat_outputs"):
        validate_document(document)


def test_rejects_missing_self_loop_and_asymmetry() -> None:
    document = lab()
    document["neighborhoods"][0] = [1]
    with pytest.raises(
        GraphConvolutionAttentionValidationError, match="including self"
    ):
        validate_document(document)
    document = lab()
    document["neighborhoods"][0] = [0]
    with pytest.raises(GraphConvolutionAttentionValidationError, match="symmetric"):
        validate_document(document)


def test_loader_rejects_duplicate_and_non_finite(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text('{"x":1,"x":2}', encoding="utf-8")
    with pytest.raises(GraphConvolutionAttentionValidationError, match="duplicate"):
        load_json(path)
    path.write_text('{"x":NaN}', encoding="utf-8")
    with pytest.raises(GraphConvolutionAttentionValidationError, match="non-finite"):
        load_json(path)
