#!/usr/bin/env python3
"""Validate and execute the deterministic NN17 scalar VAE corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "variational-autoencoder-v1"
)
PARAMETER_ORDER = [
    "encoder.mean.weight",
    "encoder.mean.bias",
    "encoder.log_variance.weight",
    "encoder.log_variance.bias",
    "decoder.weight",
    "decoder.bias",
]


class VariationalAutoencoderValidationError(ValueError):
    """Raised when an NN17 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise VariationalAutoencoderValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                VariationalAutoencoderValidationError(
                    f"non-finite JSON number: {value}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise VariationalAutoencoderValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise VariationalAutoencoderValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise VariationalAutoencoderValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise VariationalAutoencoderValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise VariationalAutoencoderValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise VariationalAutoencoderValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _affine(value: Any, context: str) -> dict[str, float]:
    affine = _object(value, {"weight", "bias"}, context)
    return {
        "weight": _number(affine["weight"], f"{context}.weight"),
        "bias": _number(affine["bias"], f"{context}.bias"),
    }


def _clean_zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-15 else value


def _forward(
    input_value: float,
    parameters: dict[str, Any],
    epsilon: float,
    beta: float,
) -> dict[str, float]:
    mean_product = _clean_zero(input_value * parameters["encoder"]["mean"]["weight"])
    mean = _clean_zero(mean_product + parameters["encoder"]["mean"]["bias"])
    log_variance_product = _clean_zero(
        input_value * parameters["encoder"]["log_variance"]["weight"]
    )
    log_variance = _clean_zero(
        log_variance_product + parameters["encoder"]["log_variance"]["bias"]
    )
    try:
        variance = math.exp(log_variance)
        standard_deviation = math.exp(0.5 * log_variance)
    except OverflowError as error:
        raise VariationalAutoencoderValidationError(
            "encoder.log_variance: exponential overflow"
        ) from error
    if not math.isfinite(variance) or not math.isfinite(standard_deviation):
        raise VariationalAutoencoderValidationError(
            "encoder.log_variance: non-finite exponential"
        )
    noise_contribution = _clean_zero(standard_deviation * epsilon)
    latent = _clean_zero(mean + noise_contribution)
    decoder_product = _clean_zero(latent * parameters["decoder"]["weight"])
    reconstruction = _clean_zero(decoder_product + parameters["decoder"]["bias"])
    error = _clean_zero(reconstruction - input_value)
    reconstruction_loss = 0.5 * error * error
    mean_squared = mean * mean
    kl = 0.5 * (mean_squared + variance - 1 - log_variance)
    weighted_kl = beta * kl
    return {
        "mean_product": mean_product,
        "mean": mean,
        "log_variance_product": log_variance_product,
        "log_variance": log_variance,
        "variance": variance,
        "standard_deviation": standard_deviation,
        "epsilon": epsilon,
        "noise_contribution": noise_contribution,
        "latent": latent,
        "decoder_product": decoder_product,
        "reconstruction": reconstruction,
        "error": error,
        "reconstruction_loss": reconstruction_loss,
        "mean_squared": mean_squared,
        "kl": kl,
        "weighted_kl": weighted_kl,
        "total_loss": reconstruction_loss + weighted_kl,
    }


def _flatten(parameters: dict[str, Any]) -> list[float]:
    return [
        parameters["encoder"]["mean"]["weight"],
        parameters["encoder"]["mean"]["bias"],
        parameters["encoder"]["log_variance"]["weight"],
        parameters["encoder"]["log_variance"]["bias"],
        parameters["decoder"]["weight"],
        parameters["decoder"]["bias"],
    ]


def _from_flat(values: list[float]) -> dict[str, Any]:
    return {
        "encoder": {
            "mean": {"weight": values[0], "bias": values[1]},
            "log_variance": {"weight": values[2], "bias": values[3]},
        },
        "decoder": {"weight": values[4], "bias": values[5]},
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    input_value = _number(document["input"], "input")
    parameters = {
        "encoder": {
            "mean": _affine(document["encoder"]["mean"], "encoder.mean"),
            "log_variance": _affine(
                document["encoder"]["log_variance"],
                "encoder.log_variance",
            ),
        },
        "decoder": _affine(document["decoder"], "decoder"),
    }
    epsilon = _number(document["sample"]["epsilon"], "sample.epsilon")
    beta = _number(document["objective"]["beta"], "objective.beta")
    learning_rate = _number(
        document["optimizer"]["learning_rate"],
        "optimizer.learning_rate",
    )
    forward = _forward(input_value, parameters, epsilon, beta)

    reconstruction_gradient = forward["error"]
    decoder_weight_gradient = _clean_zero(reconstruction_gradient * forward["latent"])
    decoder_bias_gradient = reconstruction_gradient
    latent_gradient = _clean_zero(
        reconstruction_gradient * parameters["decoder"]["weight"]
    )
    reconstruction_mean_gradient = latent_gradient
    reconstruction_log_variance_gradient = _clean_zero(
        latent_gradient * 0.5 * forward["standard_deviation"] * epsilon
    )
    kl_mean_gradient = forward["mean"]
    kl_log_variance_gradient = _clean_zero(0.5 * (forward["variance"] - 1))
    weighted_kl_mean_gradient = _clean_zero(beta * kl_mean_gradient)
    weighted_kl_log_variance_gradient = _clean_zero(beta * kl_log_variance_gradient)
    mean_gradient = _clean_zero(
        reconstruction_mean_gradient + weighted_kl_mean_gradient
    )
    log_variance_gradient = _clean_zero(
        reconstruction_log_variance_gradient + weighted_kl_log_variance_gradient
    )
    mean_weight_gradient = _clean_zero(mean_gradient * input_value)
    mean_bias_gradient = mean_gradient
    log_variance_weight_gradient = _clean_zero(log_variance_gradient * input_value)
    log_variance_bias_gradient = log_variance_gradient
    backward = {
        "reconstruction_gradient": reconstruction_gradient,
        "decoder_weight_gradient": decoder_weight_gradient,
        "decoder_bias_gradient": decoder_bias_gradient,
        "latent_gradient": latent_gradient,
        "reconstruction_mean_gradient": reconstruction_mean_gradient,
        "reconstruction_log_variance_gradient": (reconstruction_log_variance_gradient),
        "kl_mean_gradient": kl_mean_gradient,
        "kl_log_variance_gradient": kl_log_variance_gradient,
        "weighted_kl_mean_gradient": weighted_kl_mean_gradient,
        "weighted_kl_log_variance_gradient": weighted_kl_log_variance_gradient,
        "mean_gradient": mean_gradient,
        "log_variance_gradient": log_variance_gradient,
        "mean_weight_gradient": mean_weight_gradient,
        "mean_bias_gradient": mean_bias_gradient,
        "log_variance_weight_gradient": log_variance_weight_gradient,
        "log_variance_bias_gradient": log_variance_bias_gradient,
    }

    analytical = [
        mean_weight_gradient,
        mean_bias_gradient,
        log_variance_weight_gradient,
        log_variance_bias_gradient,
        decoder_weight_gradient,
        decoder_bias_gradient,
    ]
    flat_parameters = _flatten(parameters)
    audit_epsilon = 1e-6
    numerical = []
    for parameter_index in range(len(flat_parameters)):
        plus = flat_parameters[:]
        minus = flat_parameters[:]
        plus[parameter_index] += audit_epsilon
        minus[parameter_index] -= audit_epsilon
        plus_loss = _forward(
            input_value,
            _from_flat(plus),
            epsilon,
            beta,
        )["total_loss"]
        minus_loss = _forward(
            input_value,
            _from_flat(minus),
            epsilon,
            beta,
        )["total_loss"]
        numerical.append((plus_loss - minus_loss) / (2 * audit_epsilon))
    gradient_check = {
        "epsilon": audit_epsilon,
        "parameter_order": PARAMETER_ORDER,
        "analytical": analytical,
        "numerical": numerical,
        "max_absolute_error": max(
            abs(analytical_value - numerical_value)
            for analytical_value, numerical_value in zip(analytical, numerical)
        ),
    }

    updated_parameters = _from_flat(
        [
            value - learning_rate * analytical[index]
            for index, value in enumerate(flat_parameters)
        ]
    )
    post_update = _forward(
        input_value,
        updated_parameters,
        epsilon,
        beta,
    )
    return {
        "forward": forward,
        "backward": backward,
        "gradient_check": gradient_check,
        "updated_parameters": updated_parameters,
        "post_update": post_update,
    }


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise VariationalAutoencoderValidationError(
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
            raise VariationalAutoencoderValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise VariationalAutoencoderValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise VariationalAutoencoderValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise VariationalAutoencoderValidationError(
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
            "input",
            "encoder",
            "decoder",
            "sample",
            "objective",
            "optimizer",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise VariationalAutoencoderValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise VariationalAutoencoderValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise VariationalAutoencoderValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise VariationalAutoencoderValidationError(
            "absolute_tolerance: expected a positive number"
        )

    operation = _object(
        document["operation"],
        {
            "kind",
            "input_width",
            "latent_width",
            "output_width",
            "posterior",
            "sampling",
            "decoder_activation",
            "reconstruction_loss",
            "kl_prior",
            "objective",
            "trainable_parameters",
            "update",
        },
        "operation",
    )
    required_operation = {
        "kind": "scalar-variational-autoencoder-step",
        "input_width": 1,
        "latent_width": 1,
        "output_width": 1,
        "posterior": "diagonal-gaussian",
        "sampling": "reparameterization-with-saved-epsilon",
        "decoder_activation": "identity",
        "reconstruction_loss": "half-squared-error",
        "kl_prior": "standard-normal",
        "objective": "beta-vae",
        "trainable_parameters": [
            "encoder.mean",
            "encoder.log_variance",
            "decoder",
        ],
        "update": "sgd",
    }
    if operation != required_operation:
        raise VariationalAutoencoderValidationError(
            "operation: unsupported NN17 contract"
        )

    _number(document["input"], "input")
    encoder = _object(
        document["encoder"],
        {"mean", "log_variance"},
        "encoder",
    )
    _affine(encoder["mean"], "encoder.mean")
    _affine(encoder["log_variance"], "encoder.log_variance")
    _affine(document["decoder"], "decoder")
    sample = _object(document["sample"], {"epsilon"}, "sample")
    _number(sample["epsilon"], "sample.epsilon")
    objective = _object(document["objective"], {"beta"}, "objective")
    if _number(objective["beta"], "objective.beta") < 0:
        raise VariationalAutoencoderValidationError(
            "objective.beta: expected a non-negative number"
        )
    optimizer = _object(
        document["optimizer"],
        {"learning_rate"},
        "optimizer",
    )
    if _number(optimizer["learning_rate"], "optimizer.learning_rate") <= 0:
        raise VariationalAutoencoderValidationError(
            "optimizer.learning_rate: expected a positive number"
        )
    _object(
        document["expected"],
        {
            "forward",
            "backward",
            "gradient_check",
            "updated_parameters",
            "post_update",
        },
        "expected",
    )
    actual = execute_lab(document)
    _compare(document["expected"], actual, tolerance, "expected")
    if actual["gradient_check"]["max_absolute_error"] >= 1e-8:
        raise VariationalAutoencoderValidationError(
            "expected.gradient_check: analytical and numerical gradients disagree"
        )
    if actual["post_update"]["total_loss"] >= actual["forward"]["total_loss"]:
        raise VariationalAutoencoderValidationError(
            "expected.post_update.total_loss: canonical update must lower loss"
        )
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise VariationalAutoencoderValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN17 scalar variational autoencoder lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
