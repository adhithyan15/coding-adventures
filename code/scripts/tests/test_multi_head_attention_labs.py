#!/usr/bin/env python3
"""Tests for the NN14 multi-head attention fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_multi_head_attention_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_multi_head_attention_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
MultiHeadAttentionValidationError = validator.MultiHeadAttentionValidationError
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-two-head-causal-add-norm.json"


def test_checked_in_nn14_corpus_is_valid() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_blue_heads_attend_differently() -> None:
    result = execute_lab(load_json(LAB_PATH))
    blue = result["rows"][1]

    assert blue["heads"][0]["weights"] == [0.5, 0.5, 0.0]
    assert blue["heads"][0]["context"] == 1.0
    assert blue["heads"][1]["weights"][0] == pytest.approx(0.2689414213699951)
    assert blue["heads"][1]["weights"][1] == pytest.approx(0.7310585786300049)
    assert blue["heads"][1]["context"] == pytest.approx(0.7310585786300049)


def test_blue_add_and_norm_trace_is_explicit() -> None:
    result = execute_lab(load_json(LAB_PATH))
    blue = result["rows"][1]

    assert blue["concatenated"] == pytest.approx([1.0, 0.7310585786300049])
    assert blue["projected_attention"] == pytest.approx(blue["concatenated"])
    assert blue["residual_sum"] == pytest.approx([1.0, 1.7310585786300048])
    assert blue["layer_norm"]["mean"] == pytest.approx(1.3655292893150024)
    assert blue["layer_norm"]["output"] == pytest.approx(
        [-0.9999625802171532, 0.9999625802171532]
    )


def test_causal_rows_normalize_inside_each_head() -> None:
    result = execute_lab(load_json(LAB_PATH))
    for row_index, row in enumerate(result["rows"]):
        for head in row["heads"]:
            assert sum(head["weights"]) == pytest.approx(1.0)
            assert head["weights"][row_index + 1 :] == [0.0] * (2 - row_index)


def test_validator_rejects_unknown_fields_and_wrong_shapes() -> None:
    document = load_json(LAB_PATH)
    with_extra = copy.deepcopy(document)
    with_extra["heads"][0]["mystery"] = 3
    with pytest.raises(MultiHeadAttentionValidationError, match="key mismatch"):
        validate_lab(with_extra)

    wrong_shape = copy.deepcopy(document)
    wrong_shape["output_projection"] = [[1, 0]]
    with pytest.raises(MultiHeadAttentionValidationError, match="expected 2 rows"):
        validate_lab(wrong_shape)


def test_validator_rejects_drifted_expected_trace() -> None:
    document = load_json(LAB_PATH)
    document["expected"]["rows"][1]["layer_norm"]["mean"] = 999
    with pytest.raises(MultiHeadAttentionValidationError, match="expected.rows"):
        validate_lab(document)


def test_loader_rejects_duplicate_keys(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(MultiHeadAttentionValidationError, match="duplicate JSON key"):
        load_json(duplicate)


def test_loader_rejects_non_finite_numbers(tmp_path: Path) -> None:
    invalid = tmp_path / "invalid.json"
    invalid.write_text(json.dumps({"value": float("nan")}), encoding="utf-8")
    with pytest.raises(MultiHeadAttentionValidationError, match="non-finite"):
        load_json(invalid)
