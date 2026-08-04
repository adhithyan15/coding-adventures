#!/usr/bin/env python3
"""Validate and execute the deterministic NN18 one-dimensional GAN corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "one-dimensional-gan-v1"
)
DISCRIMINATOR_PARAMETER_ORDER = [
    "discriminator.weight",
    "discriminator.bias",
]
GENERATOR_PARAMETER_ORDER = ["generator.weight", "generator.bias"]


class OneDimensionalGanValidationError(ValueError):
    """Raised when an NN18 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise OneDimensionalGanValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                OneDimensionalGanValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise OneDimensionalGanValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise OneDimensionalGanValidationError("top-level JSON value must be an object")
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise OneDimensionalGanValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise OneDimensionalGanValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise OneDimensionalGanValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise OneDimensionalGanValidationError(f"{context}: expected a finite number")
    return result


def _affine(value: Any, context: str) -> dict[str, float]:
    affine = _object(value, {"weight", "bias"}, context)
    return {
        "weight": _number(affine["weight"], f"{context}.weight"),
        "bias": _number(affine["bias"], f"{context}.bias"),
    }


def _sigmoid(value: float) -> float:
    if value >= 0:
        negative = math.exp(-value)
        return 1 / (1 + negative)
    positive = math.exp(value)
    return positive / (1 + positive)


def _state(
    real_sample: float,
    noise: float,
    generator: dict[str, float],
    discriminator: dict[str, float],
) -> dict[str, float]:
    generator_product = noise * generator["weight"]
    fake_sample = generator_product + generator["bias"]
    real_logit = discriminator["weight"] * real_sample + discriminator["bias"]
    fake_logit = discriminator["weight"] * fake_sample + discriminator["bias"]
    real_probability = _sigmoid(real_logit)
    fake_probability = _sigmoid(fake_logit)
    if not 0 < real_probability < 1 or not 0 < fake_probability < 1:
        raise OneDimensionalGanValidationError(
            "canonical logits must produce open-interval probabilities"
        )
    discriminator_loss = -0.5 * (
        math.log(real_probability) + math.log(1 - fake_probability)
    )
    generator_loss = -math.log(fake_probability)
    return {
        "generator_product": generator_product,
        "fake_sample": fake_sample,
        "real_logit": real_logit,
        "real_probability": real_probability,
        "fake_logit": fake_logit,
        "fake_probability": fake_probability,
        "discriminator_loss": discriminator_loss,
        "generator_loss": generator_loss,
    }


def _gradient_check(
    parameters: list[float],
    analytical: list[float],
    parameter_order: list[str],
    loss_function: Any,
) -> dict[str, Any]:
    epsilon = 1e-6
    numerical = []
    for index in range(len(parameters)):
        plus = parameters[:]
        minus = parameters[:]
        plus[index] += epsilon
        minus[index] -= epsilon
        numerical.append((loss_function(plus) - loss_function(minus)) / (2 * epsilon))
    return {
        "epsilon": epsilon,
        "parameter_order": parameter_order,
        "analytical": analytical,
        "numerical": numerical,
        "max_absolute_error": max(
            abs(analytical_value - numerical_value)
            for analytical_value, numerical_value in zip(analytical, numerical)
        ),
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    real_sample = _number(document["real_sample"], "real_sample")
    noise = _number(document["noise"], "noise")
    generator = _affine(document["generator"], "generator")
    discriminator = _affine(document["discriminator"], "discriminator")
    discriminator_learning_rate = _number(
        document["optimizer"]["discriminator_learning_rate"],
        "optimizer.discriminator_learning_rate",
    )
    generator_learning_rate = _number(
        document["optimizer"]["generator_learning_rate"],
        "optimizer.generator_learning_rate",
    )

    initial = _state(real_sample, noise, generator, discriminator)
    real_logit_gradient = 0.5 * (initial["real_probability"] - 1)
    fake_logit_gradient = 0.5 * initial["fake_probability"]
    discriminator_weight_gradient = (
        real_logit_gradient * real_sample + fake_logit_gradient * initial["fake_sample"]
    )
    discriminator_bias_gradient = real_logit_gradient + fake_logit_gradient
    discriminator_backward = {
        "real_logit_gradient": real_logit_gradient,
        "fake_logit_gradient": fake_logit_gradient,
        "fake_value_gradient": 0.0,
        "weight_gradient": discriminator_weight_gradient,
        "bias_gradient": discriminator_bias_gradient,
    }
    updated_discriminator = {
        "weight": discriminator["weight"]
        - discriminator_learning_rate * discriminator_weight_gradient,
        "bias": discriminator["bias"]
        - discriminator_learning_rate * discriminator_bias_gradient,
    }
    after_discriminator = _state(
        real_sample,
        noise,
        generator,
        updated_discriminator,
    )
    discriminator_gradient_check = _gradient_check(
        [discriminator["weight"], discriminator["bias"]],
        [discriminator_weight_gradient, discriminator_bias_gradient],
        DISCRIMINATOR_PARAMETER_ORDER,
        lambda values: _state(
            real_sample,
            noise,
            generator,
            {"weight": values[0], "bias": values[1]},
        )["discriminator_loss"],
    )

    generator_fake_logit_gradient = after_discriminator["fake_probability"] - 1
    fake_value_gradient = (
        generator_fake_logit_gradient * updated_discriminator["weight"]
    )
    generator_weight_gradient = fake_value_gradient * noise
    generator_bias_gradient = fake_value_gradient
    generator_backward = {
        "fake_logit_gradient": generator_fake_logit_gradient,
        "fake_value_gradient": fake_value_gradient,
        "weight_gradient": generator_weight_gradient,
        "bias_gradient": generator_bias_gradient,
    }
    updated_generator = {
        "weight": generator["weight"]
        - generator_learning_rate * generator_weight_gradient,
        "bias": generator["bias"] - generator_learning_rate * generator_bias_gradient,
    }
    after_generator = _state(
        real_sample,
        noise,
        updated_generator,
        updated_discriminator,
    )
    generator_gradient_check = _gradient_check(
        [generator["weight"], generator["bias"]],
        [generator_weight_gradient, generator_bias_gradient],
        GENERATOR_PARAMETER_ORDER,
        lambda values: _state(
            real_sample,
            noise,
            {"weight": values[0], "bias": values[1]},
            updated_discriminator,
        )["generator_loss"],
    )
    return {
        "initial": initial,
        "discriminator_backward": discriminator_backward,
        "updated_discriminator": updated_discriminator,
        "after_discriminator": after_discriminator,
        "discriminator_gradient_check": discriminator_gradient_check,
        "generator_backward": generator_backward,
        "updated_generator": updated_generator,
        "after_generator": after_generator,
        "generator_gradient_check": generator_gradient_check,
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise OneDimensionalGanValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected),
            float(actual),
            rel_tol=0,
            abs_tol=tolerance,
        ):
            raise OneDimensionalGanValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise OneDimensionalGanValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise OneDimensionalGanValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise OneDimensionalGanValidationError(
            f"{context}: expected {expected!r}, got {actual!r}"
        )


def validate_lab(document: dict[str, Any]) -> dict[str, Any]:
    _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "real_sample",
            "noise",
            "generator",
            "discriminator",
            "optimizer",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise OneDimensionalGanValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise OneDimensionalGanValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise OneDimensionalGanValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise OneDimensionalGanValidationError(
            "absolute_tolerance: expected a positive number"
        )

    operation = _object(
        document["operation"],
        {
            "kind",
            "generator",
            "discriminator",
            "discriminator_loss",
            "generator_loss",
            "schedule",
            "fake_handling",
            "update",
        },
        "operation",
    )
    required_operation = {
        "kind": "one-dimensional-gan-round",
        "generator": "scalar-affine",
        "discriminator": "sigmoid-of-scalar-affine",
        "discriminator_loss": "mean-binary-cross-entropy",
        "generator_loss": "non-saturating-binary-cross-entropy",
        "schedule": "discriminator-then-generator",
        "fake_handling": "detach-during-discriminator-step",
        "update": "sgd",
    }
    if operation != required_operation:
        raise OneDimensionalGanValidationError("operation: unsupported NN18 contract")

    _number(document["real_sample"], "real_sample")
    _number(document["noise"], "noise")
    _affine(document["generator"], "generator")
    _affine(document["discriminator"], "discriminator")
    optimizer = _object(
        document["optimizer"],
        {"discriminator_learning_rate", "generator_learning_rate"},
        "optimizer",
    )
    for key in ("discriminator_learning_rate", "generator_learning_rate"):
        if _number(optimizer[key], f"optimizer.{key}") <= 0:
            raise OneDimensionalGanValidationError(
                f"optimizer.{key}: expected a positive number"
            )
    _object(
        document["expected"],
        {
            "initial",
            "discriminator_backward",
            "updated_discriminator",
            "after_discriminator",
            "discriminator_gradient_check",
            "generator_backward",
            "updated_generator",
            "after_generator",
            "generator_gradient_check",
        },
        "expected",
    )
    actual = execute_lab(document)
    _compare(document["expected"], actual, tolerance, "expected")
    for player in ("discriminator", "generator"):
        if actual[f"{player}_gradient_check"]["max_absolute_error"] >= 1e-8:
            raise OneDimensionalGanValidationError(
                f"expected.{player}_gradient_check: gradients disagree"
            )
    if (
        actual["after_discriminator"]["discriminator_loss"]
        >= actual["initial"]["discriminator_loss"]
    ):
        raise OneDimensionalGanValidationError(
            "expected.after_discriminator: discriminator step must lower its loss"
        )
    if (
        actual["after_generator"]["generator_loss"]
        >= actual["after_discriminator"]["generator_loss"]
    ):
        raise OneDimensionalGanValidationError(
            "expected.after_generator: generator step must lower its loss"
        )
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise OneDimensionalGanValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN18 one-dimensional GAN lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
