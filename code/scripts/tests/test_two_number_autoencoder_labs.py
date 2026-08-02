#!/usr/bin/env python3
"""Tests for the NN16 two-number autoencoder fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_two_number_autoencoder_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_two_number_autoencoder_labs", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
TwoNumberAutoencoderValidationError = validator.TwoNumberAutoencoderValidationError
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-linear-bottleneck-step.json"


def test_checked_in_nn16_corpus_is_valid() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_forward_trace_compresses_two_values_into_one() -> None:
    result = execute_lab(load_json(LAB_PATH))
    forward = result["forward"]

    assert forward["encoder_products"] == pytest.approx([1.0, 0.25])
    assert forward["bottleneck"] == pytest.approx(1.25)
    assert forward["decoder_products"] == pytest.approx([1.5, -1.0])
    assert forward["reconstruction"] == pytest.approx([1.6, -1.2])
    assert forward["loss"] == pytest.approx(0.1)


def test_both_reconstruction_errors_meet_at_bottleneck() -> None:
    backward = execute_lab(load_json(LAB_PATH))["backward"]

    assert backward["reconstruction_gradients"] == pytest.approx([-0.4, -0.2])
    assert backward["bottleneck_gradient_contributions"] == pytest.approx([-0.48, 0.16])
    assert backward["bottleneck_gradient"] == pytest.approx(-0.32)
    assert backward["encoder_weight_gradients"] == pytest.approx([-0.64, 0.32])


def test_central_finite_differences_audit_all_parameters() -> None:
    result = execute_lab(load_json(LAB_PATH))
    check = result["gradient_check"]

    assert len(check["parameter_order"]) == 7
    assert check["numerical"] == pytest.approx(check["analytical"], abs=1e-8)
    assert check["max_absolute_error"] < 1e-8


def test_one_sgd_step_lowers_reconstruction_loss() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert result["updated_parameters"]["encoder"]["weights"] == pytest.approx(
        [0.564, -0.282]
    )
    assert result["post_update"]["bottleneck"] == pytest.approx(1.442)
    assert result["post_update"]["reconstruction"] == pytest.approx([1.9425, -1.29755])
    assert result["post_update"]["loss"] == pytest.approx(0.04592112625)
    assert result["post_update"]["loss"] < result["forward"]["loss"]


def test_validator_rejects_unknown_fields_and_wrong_shapes() -> None:
    document = load_json(LAB_PATH)
    with_extra = copy.deepcopy(document)
    with_extra["encoder"]["mystery"] = 3
    with pytest.raises(TwoNumberAutoencoderValidationError, match="key mismatch"):
        validate_lab(with_extra)

    wrong_shape = copy.deepcopy(document)
    wrong_shape["decoder"]["weights"] = [1.2]
    with pytest.raises(TwoNumberAutoencoderValidationError, match="expected 2 numbers"):
        validate_lab(wrong_shape)


def test_validator_rejects_drifted_expected_trace() -> None:
    document = load_json(LAB_PATH)
    document["expected"]["forward"]["bottleneck"] = 999
    with pytest.raises(TwoNumberAutoencoderValidationError, match="expected.forward"):
        validate_lab(document)


def test_loader_rejects_duplicate_keys(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(TwoNumberAutoencoderValidationError, match="duplicate JSON key"):
        load_json(duplicate)


def test_loader_rejects_non_finite_numbers(tmp_path: Path) -> None:
    invalid = tmp_path / "invalid.json"
    invalid.write_text(json.dumps({"value": float("nan")}), encoding="utf-8")
    with pytest.raises(TwoNumberAutoencoderValidationError, match="non-finite"):
        load_json(invalid)
