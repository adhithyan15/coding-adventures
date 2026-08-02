#!/usr/bin/env python3
"""Tests for the NN17 scalar variational autoencoder fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = (
    REPO_ROOT / "code" / "scripts" / "validate_variational_autoencoder_labs.py"
)
SPEC = importlib.util.spec_from_file_location(
    "validate_variational_autoencoder_labs",
    MODULE_PATH,
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
VariationalAutoencoderValidationError = validator.VariationalAutoencoderValidationError
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-saved-noise-kl-step.json"


def test_checked_in_nn17_corpus_is_valid() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_saved_epsilon_makes_the_sample_reproducible() -> None:
    result = execute_lab(load_json(LAB_PATH))["forward"]

    assert result["mean"] == pytest.approx(0.4)
    assert result["standard_deviation"] == pytest.approx(1)
    assert result["epsilon"] == pytest.approx(0.5)
    assert result["noise_contribution"] == pytest.approx(0.5)
    assert result["latent"] == pytest.approx(0.9)
    assert result["reconstruction"] == pytest.approx(0.9)


def test_objective_keeps_reconstruction_and_kl_visible() -> None:
    forward = execute_lab(load_json(LAB_PATH))["forward"]

    assert forward["reconstruction_loss"] == pytest.approx(0.005)
    assert forward["kl"] == pytest.approx(0.08)
    assert forward["weighted_kl"] == pytest.approx(0.008)
    assert forward["total_loss"] == pytest.approx(0.013)


def test_reconstruction_and_kl_routes_add_at_encoder_outputs() -> None:
    backward = execute_lab(load_json(LAB_PATH))["backward"]

    assert backward["reconstruction_mean_gradient"] == pytest.approx(-0.1)
    assert backward["weighted_kl_mean_gradient"] == pytest.approx(0.04)
    assert backward["mean_gradient"] == pytest.approx(-0.06)
    assert backward["reconstruction_log_variance_gradient"] == pytest.approx(-0.025)
    assert backward["weighted_kl_log_variance_gradient"] == pytest.approx(0)
    assert backward["log_variance_gradient"] == pytest.approx(-0.025)


def test_central_finite_differences_hold_saved_noise_fixed() -> None:
    check = execute_lab(load_json(LAB_PATH))["gradient_check"]

    assert len(check["parameter_order"]) == 6
    assert check["numerical"] == pytest.approx(check["analytical"], abs=1e-8)
    assert check["max_absolute_error"] < 1e-8


def test_one_sgd_step_lowers_total_vae_objective() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert result["updated_parameters"]["encoder"]["mean"] == pytest.approx(
        {"weight": 0.406, "bias": 0.006}
    )
    assert result["post_update"]["mean"] == pytest.approx(0.412)
    assert result["post_update"]["latent"] == pytest.approx(0.9132515638028976)
    assert result["post_update"]["reconstruction"] == pytest.approx(0.9314708278771237)
    assert result["post_update"]["total_loss"] == pytest.approx(0.01083594975889346)
    assert result["post_update"]["total_loss"] < result["forward"]["total_loss"]


def test_validator_rejects_unknown_fields_and_negative_beta() -> None:
    document = load_json(LAB_PATH)
    with_extra = copy.deepcopy(document)
    with_extra["sample"]["mystery"] = 3
    with pytest.raises(VariationalAutoencoderValidationError, match="key mismatch"):
        validate_lab(with_extra)

    negative_beta = copy.deepcopy(document)
    negative_beta["objective"]["beta"] = -1
    with pytest.raises(
        VariationalAutoencoderValidationError,
        match="non-negative",
    ):
        validate_lab(negative_beta)


def test_validator_rejects_drifted_expected_trace() -> None:
    document = load_json(LAB_PATH)
    document["expected"]["forward"]["latent"] = 999
    with pytest.raises(VariationalAutoencoderValidationError, match="expected.forward"):
        validate_lab(document)


def test_loader_rejects_duplicate_keys(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(
        VariationalAutoencoderValidationError,
        match="duplicate JSON key",
    ):
        load_json(duplicate)


def test_loader_rejects_non_finite_numbers(tmp_path: Path) -> None:
    invalid = tmp_path / "invalid.json"
    invalid.write_text(json.dumps({"value": float("nan")}), encoding="utf-8")
    with pytest.raises(VariationalAutoencoderValidationError, match="non-finite"):
        load_json(invalid)
