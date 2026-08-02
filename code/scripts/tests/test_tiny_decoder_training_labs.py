#!/usr/bin/env python3
"""Tests for the NN15 tiny decoder training fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_tiny_decoder_training_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_tiny_decoder_training_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
TinyDecoderTrainingValidationError = validator.TinyDecoderTrainingValidationError
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-two-position-next-token-step.json"


def test_checked_in_nn15_corpus_is_valid() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_sequence_shift_builds_two_causal_training_positions() -> None:
    document = load_json(LAB_PATH)
    result = execute_lab(document)

    assert document["input_tokens"] == ["red", "blue"]
    assert document["target_tokens"] == ["blue", "purple"]
    assert [row["causal_prefix"] for row in result["rows"]] == [
        ["red"],
        ["red", "blue"],
    ]


def test_forward_trace_reproduces_logits_softmax_and_loss() -> None:
    result = execute_lab(load_json(LAB_PATH))
    first, second = result["rows"]

    assert first["logit_products"] == [[1.0, 0.0], [0.0, 0.0], [-1.0, 0.0]]
    assert first["logits"] == [1.0, 0.0, -1.0]
    assert sum(first["probabilities"]) == pytest.approx(1.0)
    assert first["target_probability"] == pytest.approx(0.24472847105479764)
    assert second["target_probability"] == pytest.approx(0.09003057317038046)
    assert result["mean_loss"] == pytest.approx(1.9076059644443801)


def test_backward_trace_reduces_shared_gradients() -> None:
    result = execute_lab(load_json(LAB_PATH))
    first, second = result["rows"]

    assert first["unembedding_gradient_contribution"][1] == [0.0, 0.0, 0.0]
    assert second["unembedding_gradient_contribution"][0] == [0.0, 0.0, 0.0]
    assert result["unembedding_gradient"][0] == pytest.approx(
        [0.3326204778874109, -0.37763576447260117, 0.04501528658519023]
    )
    assert result["unembedding_gradient"][1] == pytest.approx(
        [0.12236423552739882, 0.3326204778874109, -0.4549847134148098]
    )
    assert first["state_gradient"] == pytest.approx(
        [
            0.2876051913022207,
            -0.4226510510577914,
        ]
    )


def test_one_sgd_step_lowers_mean_loss() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert result["updated_bias"] == pytest.approx(
        [
            -0.22749235670740486,
            0.022507643292595136,
            0.20498471341480978,
        ]
    )
    assert result["post_update_mean_loss"] == pytest.approx(1.456094285138867)
    assert result["post_update_mean_loss"] < result["mean_loss"]


def test_central_finite_differences_audit_shared_head_gradients() -> None:
    result = execute_lab(load_json(LAB_PATH))
    check = result["gradient_check"]

    assert check["epsilon"] == 1e-6
    assert check["numerical_unembedding_gradient"][0] == pytest.approx(
        result["unembedding_gradient"][0], abs=1e-8
    )
    assert check["numerical_unembedding_gradient"][1] == pytest.approx(
        result["unembedding_gradient"][1], abs=1e-8
    )
    assert check["numerical_bias_gradient"] == pytest.approx(
        result["bias_gradient"], abs=1e-8
    )
    assert check["max_absolute_error"] < 1e-8


def test_validator_rejects_unknown_fields_and_broken_shift() -> None:
    document = load_json(LAB_PATH)
    with_extra = copy.deepcopy(document)
    with_extra["mystery"] = 3
    with pytest.raises(TinyDecoderTrainingValidationError, match="key mismatch"):
        validate_lab(with_extra)

    broken_shift = copy.deepcopy(document)
    broken_shift["target_tokens"] = ["purple", "blue"]
    with pytest.raises(TinyDecoderTrainingValidationError, match="sequence shift"):
        validate_lab(broken_shift)


def test_validator_rejects_drifted_expected_trace() -> None:
    document = load_json(LAB_PATH)
    document["expected"]["post_update_mean_loss"] = 999
    with pytest.raises(TinyDecoderTrainingValidationError, match="expected"):
        validate_lab(document)


def test_loader_rejects_duplicate_keys(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(TinyDecoderTrainingValidationError, match="duplicate JSON key"):
        load_json(duplicate)


def test_loader_rejects_non_finite_numbers(tmp_path: Path) -> None:
    invalid = tmp_path / "invalid.json"
    invalid.write_text(json.dumps({"value": float("nan")}), encoding="utf-8")
    with pytest.raises(TinyDecoderTrainingValidationError, match="non-finite"):
        load_json(invalid)
