#!/usr/bin/env python3
"""Tests for the NN19 one-dimensional diffusion fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = (
    REPO_ROOT / "code" / "scripts" / "validate_one_dimensional_diffusion_labs.py"
)
SPEC = importlib.util.spec_from_file_location(
    "validate_one_dimensional_diffusion_labs",
    MODULE_PATH,
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
OneDimensionalDiffusionValidationError = (
    validator.OneDimensionalDiffusionValidationError
)
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-two-level-forward-and-reverse.json"


def test_canonical_corpus_validates() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_forward_schedule_trades_signal_for_saved_noise() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert [row["alpha_bar"] for row in result["forward_steps"]] == pytest.approx(
        [0.64, 0.36]
    )
    assert [row["signal_scale"] for row in result["forward_steps"]] == pytest.approx(
        [0.8, 0.6]
    )
    assert [row["noise_scale"] for row in result["forward_steps"]] == pytest.approx(
        [0.6, 0.8]
    )
    assert [row["noisy_sample"] for row in result["forward_steps"]] == pytest.approx(
        [0.5, 0.2]
    )


def test_initial_noise_predictions_share_one_mean_objective() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert [row["predicted_noise"] for row in result["initial_denoising"]] == [
        0,
        0,
    ]
    assert [row["error"] for row in result["initial_denoising"]] == [0.5, 0.5]
    assert result["initial_mean_loss"] == pytest.approx(0.125)


def test_both_timesteps_reduce_into_shared_gradients() -> None:
    result = execute_lab(load_json(LAB_PATH))
    backward = result["backward"]

    assert [
        row["sample_weight_contribution"] for row in backward["per_step"]
    ] == pytest.approx([0.125, 0.05])
    assert backward["sample_weight_gradient"] == pytest.approx(0.175)
    assert backward["timestep_weight_gradient"] == pytest.approx(0.375)
    assert backward["bias_gradient"] == pytest.approx(0.5)


def test_gradient_audit_matches_three_analytical_slopes() -> None:
    result = execute_lab(load_json(LAB_PATH))
    audit = result["gradient_check"]

    assert audit["parameter_order"] == [
        "denoiser.sample_weight",
        "denoiser.timestep_weight",
        "denoiser.bias",
    ]
    assert audit["analytical"] == pytest.approx([0.175, 0.375, 0.5])
    assert audit["max_absolute_error"] < 1e-8


def test_sgd_update_lowers_noise_prediction_loss() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert result["updated_denoiser"] == pytest.approx(
        {
            "sample_weight": -0.0875,
            "timestep_weight": -0.1875,
            "bias": -0.25,
        }
    )
    assert [
        row["predicted_noise"] for row in result["post_update_denoising"]
    ] == pytest.approx([-0.3875, -0.455])
    assert result["post_update_mean_loss"] == pytest.approx(0.0036703125)
    assert result["post_update_mean_loss"] < result["initial_mean_loss"]


def test_reverse_mean_reconstructs_the_clean_sample() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert [row["t"] for row in result["reverse_steps"]] == [2, 1]
    assert [row["output_mean"] for row in result["reverse_steps"]] == pytest.approx(
        [0.5984375, 1.0451318359375]
    )
    assert result["final_reconstruction"] == pytest.approx(1.0451318359375)
    assert result["final_absolute_error"] == pytest.approx(0.0451318359375)


def test_unknown_fields_are_rejected() -> None:
    document = copy.deepcopy(load_json(LAB_PATH))
    document["mystery"] = True

    with pytest.raises(OneDimensionalDiffusionValidationError, match="key mismatch"):
        validate_lab(document)


def test_invalid_schedule_order_is_rejected() -> None:
    document = copy.deepcopy(load_json(LAB_PATH))
    document["schedule"][1]["t"] = 3

    with pytest.raises(
        OneDimensionalDiffusionValidationError,
        match="consecutive timesteps",
    ):
        validate_lab(document)


def test_mutated_expected_trace_is_rejected() -> None:
    document = copy.deepcopy(load_json(LAB_PATH))
    document["expected"]["final_reconstruction"] = 999

    with pytest.raises(
        OneDimensionalDiffusionValidationError,
        match="expected.final_reconstruction",
    ):
        validate_lab(document)


def test_loader_rejects_duplicate_and_non_finite_json(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}')
    with pytest.raises(OneDimensionalDiffusionValidationError, match="duplicate"):
        load_json(duplicate)

    non_finite = tmp_path / "non-finite.json"
    non_finite.write_text(json.dumps({"value": float("nan")}))
    with pytest.raises(OneDimensionalDiffusionValidationError, match="non-finite"):
        load_json(non_finite)
