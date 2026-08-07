#!/usr/bin/env python3
"""Tests for the NN18 one-dimensional GAN fixture validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPO_ROOT / "code" / "scripts" / "validate_one_dimensional_gan_labs.py"
SPEC = importlib.util.spec_from_file_location(
    "validate_one_dimensional_gan_labs",
    MODULE_PATH,
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

DEFAULT_FIXTURE_ROOT = validator.DEFAULT_FIXTURE_ROOT
OneDimensionalGanValidationError = validator.OneDimensionalGanValidationError
execute_lab = validator.execute_lab
load_json = validator.load_json
validate_fixture_root = validator.validate_fixture_root
validate_lab = validator.validate_lab

LAB_PATH = DEFAULT_FIXTURE_ROOT / "labs" / "00-discriminator-generator-round.json"


def test_checked_in_nn18_corpus_is_valid() -> None:
    assert validate_fixture_root(DEFAULT_FIXTURE_ROOT) == 1


def test_generator_maps_saved_noise_to_one_fake_point() -> None:
    initial = execute_lab(load_json(LAB_PATH))["initial"]

    assert initial["generator_product"] == pytest.approx(0.2)
    assert initial["fake_sample"] == pytest.approx(0.2)
    assert initial["real_probability"] == pytest.approx(0.7310585786300049)
    assert initial["fake_probability"] == pytest.approx(0.549833997312478)


def test_discriminator_combines_real_and_detached_fake_routes() -> None:
    result = execute_lab(load_json(LAB_PATH))
    backward = result["discriminator_backward"]

    assert backward["real_logit_gradient"] == pytest.approx(-0.13447071068499755)
    assert backward["fake_logit_gradient"] == pytest.approx(0.274916998656239)
    assert backward["fake_value_gradient"] == 0
    assert backward["weight_gradient"] == pytest.approx(-0.07948731095374975)
    assert backward["bias_gradient"] == pytest.approx(0.14044628797124142)
    assert (
        result["after_discriminator"]["discriminator_loss"]
        < result["initial"]["discriminator_loss"]
    )


def test_generator_uses_updated_frozen_discriminator_input_gradient() -> None:
    result = execute_lab(load_json(LAB_PATH))
    backward = result["generator_backward"]

    assert backward["fake_logit_gradient"] == pytest.approx(-0.46562292571326525)
    assert backward["fake_value_gradient"] == pytest.approx(-0.48412848285494775)
    assert backward["weight_gradient"] == pytest.approx(-0.48412848285494775)
    assert backward["bias_gradient"] == pytest.approx(-0.48412848285494775)


def test_counter_move_lowers_generator_loss_and_raises_discriminator_loss() -> None:
    result = execute_lab(load_json(LAB_PATH))

    assert result["after_generator"]["fake_sample"] == pytest.approx(
        0.44206424142747386
    )
    assert result["after_generator"]["fake_probability"] == pytest.approx(
        0.5961407441300886
    )
    assert (
        result["after_generator"]["generator_loss"]
        < result["after_discriminator"]["generator_loss"]
    )
    assert (
        result["after_generator"]["discriminator_loss"]
        > result["after_discriminator"]["discriminator_loss"]
    )


def test_both_player_specific_gradient_audits_pass() -> None:
    result = execute_lab(load_json(LAB_PATH))

    for player in ("discriminator", "generator"):
        check = result[f"{player}_gradient_check"]
        assert len(check["parameter_order"]) == 2
        assert check["numerical"] == pytest.approx(check["analytical"], abs=1e-8)
        assert check["max_absolute_error"] < 1e-8


def test_validator_rejects_unknown_fields_and_bad_learning_rate() -> None:
    document = load_json(LAB_PATH)
    with_extra = copy.deepcopy(document)
    with_extra["generator"]["mystery"] = 3
    with pytest.raises(OneDimensionalGanValidationError, match="key mismatch"):
        validate_lab(with_extra)

    bad_rate = copy.deepcopy(document)
    bad_rate["optimizer"]["generator_learning_rate"] = 0
    with pytest.raises(OneDimensionalGanValidationError, match="positive"):
        validate_lab(bad_rate)


def test_validator_rejects_drifted_expected_trace() -> None:
    document = load_json(LAB_PATH)
    document["expected"]["after_generator"]["fake_sample"] = 999
    with pytest.raises(
        OneDimensionalGanValidationError, match="expected.after_generator"
    ):
        validate_lab(document)


def test_loader_rejects_duplicate_keys(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"schema_version": 1, "schema_version": 1}', encoding="utf-8")
    with pytest.raises(OneDimensionalGanValidationError, match="duplicate JSON key"):
        load_json(duplicate)


def test_loader_rejects_non_finite_numbers(tmp_path: Path) -> None:
    invalid = tmp_path / "invalid.json"
    invalid.write_text(json.dumps({"value": float("nan")}), encoding="utf-8")
    with pytest.raises(OneDimensionalGanValidationError, match="non-finite"):
        load_json(invalid)
