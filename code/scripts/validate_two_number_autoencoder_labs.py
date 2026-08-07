#!/usr/bin/env python3
"""Validate and execute the deterministic NN16 two-number autoencoder corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "two-number-autoencoder-v1"
)
PARAMETER_ORDER = [
    "encoder.weights[0]",
    "encoder.weights[1]",
    "encoder.bias",
    "decoder.weights[0]",
    "decoder.weights[1]",
    "decoder.bias[0]",
    "decoder.bias[1]",
]


class TwoNumberAutoencoderValidationError(ValueError):
    """Raised when an NN16 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TwoNumberAutoencoderValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                TwoNumberAutoencoderValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TwoNumberAutoencoderValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise TwoNumberAutoencoderValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TwoNumberAutoencoderValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise TwoNumberAutoencoderValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TwoNumberAutoencoderValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise TwoNumberAutoencoderValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _vector(value: Any, length: int, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise TwoNumberAutoencoderValidationError(
            f"{context}: expected {length} numbers"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _clean_zero(value: float) -> float:
    return 0.0 if abs(value) < 1e-15 else value


def _forward(
    input_values: list[float],
    encoder_weights: list[float],
    encoder_bias: float,
    decoder_weights: list[float],
    decoder_bias: list[float],
) -> dict[str, Any]:
    encoder_products = [
        _clean_zero(value * encoder_weights[index])
        for index, value in enumerate(input_values)
    ]
    bottleneck = _clean_zero(sum(encoder_products) + encoder_bias)
    decoder_products = [_clean_zero(bottleneck * weight) for weight in decoder_weights]
    reconstruction = [
        _clean_zero(product + decoder_bias[index])
        for index, product in enumerate(decoder_products)
    ]
    errors = [
        _clean_zero(value - input_values[index])
        for index, value in enumerate(reconstruction)
    ]
    squared_errors = [value * value for value in errors]
    return {
        "encoder_products": encoder_products,
        "bottleneck": bottleneck,
        "decoder_products": decoder_products,
        "reconstruction": reconstruction,
        "errors": errors,
        "squared_errors": squared_errors,
        "loss": sum(squared_errors) / 2,
    }


def _unpack_parameters(
    parameters: list[float],
) -> tuple[list[float], float, list[float], list[float]]:
    return parameters[:2], parameters[2], parameters[3:5], parameters[5:7]


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    input_values = _vector(document["input"], 2, "input")
    encoder_weights = _vector(document["encoder"]["weights"], 2, "encoder.weights")
    encoder_bias = _number(document["encoder"]["bias"], "encoder.bias")
    decoder_weights = _vector(document["decoder"]["weights"], 2, "decoder.weights")
    decoder_bias = _vector(document["decoder"]["bias"], 2, "decoder.bias")
    learning_rate = _number(
        document["optimizer"]["learning_rate"], "optimizer.learning_rate"
    )
    forward = _forward(
        input_values,
        encoder_weights,
        encoder_bias,
        decoder_weights,
        decoder_bias,
    )

    reconstruction_gradients = forward["errors"][:]
    decoder_weight_gradients = [
        _clean_zero(gradient * forward["bottleneck"])
        for gradient in reconstruction_gradients
    ]
    decoder_bias_gradients = reconstruction_gradients[:]
    bottleneck_gradient_contributions = [
        _clean_zero(gradient * decoder_weights[index])
        for index, gradient in enumerate(reconstruction_gradients)
    ]
    bottleneck_gradient = _clean_zero(sum(bottleneck_gradient_contributions))
    encoder_weight_gradients = [
        _clean_zero(bottleneck_gradient * value) for value in input_values
    ]
    encoder_bias_gradient = bottleneck_gradient
    backward = {
        "reconstruction_gradients": reconstruction_gradients,
        "decoder_weight_gradients": decoder_weight_gradients,
        "decoder_bias_gradients": decoder_bias_gradients,
        "bottleneck_gradient_contributions": bottleneck_gradient_contributions,
        "bottleneck_gradient": bottleneck_gradient,
        "encoder_weight_gradients": encoder_weight_gradients,
        "encoder_bias_gradient": encoder_bias_gradient,
    }

    analytical = [
        *encoder_weight_gradients,
        encoder_bias_gradient,
        *decoder_weight_gradients,
        *decoder_bias_gradients,
    ]
    parameters = [
        *encoder_weights,
        encoder_bias,
        *decoder_weights,
        *decoder_bias,
    ]
    epsilon = 1e-6
    numerical = []
    for parameter_index in range(len(parameters)):
        plus = parameters[:]
        minus = parameters[:]
        plus[parameter_index] += epsilon
        minus[parameter_index] -= epsilon
        plus_forward = _forward(input_values, *_unpack_parameters(plus))
        minus_forward = _forward(input_values, *_unpack_parameters(minus))
        numerical.append((plus_forward["loss"] - minus_forward["loss"]) / (2 * epsilon))
    gradient_check = {
        "epsilon": epsilon,
        "parameter_order": PARAMETER_ORDER,
        "analytical": analytical,
        "numerical": numerical,
        "max_absolute_error": max(
            abs(analytical_value - numerical_value)
            for analytical_value, numerical_value in zip(analytical, numerical)
        ),
    }

    updated_encoder_weights = [
        value - learning_rate * encoder_weight_gradients[index]
        for index, value in enumerate(encoder_weights)
    ]
    updated_encoder_bias = encoder_bias - learning_rate * encoder_bias_gradient
    updated_decoder_weights = [
        value - learning_rate * decoder_weight_gradients[index]
        for index, value in enumerate(decoder_weights)
    ]
    updated_decoder_bias = [
        value - learning_rate * decoder_bias_gradients[index]
        for index, value in enumerate(decoder_bias)
    ]
    updated_parameters = {
        "encoder": {
            "weights": updated_encoder_weights,
            "bias": updated_encoder_bias,
        },
        "decoder": {
            "weights": updated_decoder_weights,
            "bias": updated_decoder_bias,
        },
    }
    post_update = _forward(
        input_values,
        updated_encoder_weights,
        updated_encoder_bias,
        updated_decoder_weights,
        updated_decoder_bias,
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
            raise TwoNumberAutoencoderValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise TwoNumberAutoencoderValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise TwoNumberAutoencoderValidationError(f"{context}: length mismatch")
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise TwoNumberAutoencoderValidationError(f"{context}: key mismatch")
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise TwoNumberAutoencoderValidationError(
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
            "optimizer",
            "expected",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise TwoNumberAutoencoderValidationError("schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise TwoNumberAutoencoderValidationError(f"{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise TwoNumberAutoencoderValidationError(
            "concepts: expected unique non-empty strings"
        )
    tolerance = _number(document["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise TwoNumberAutoencoderValidationError(
            "absolute_tolerance: expected a positive number"
        )

    operation = _object(
        document["operation"],
        {
            "kind",
            "input_width",
            "bottleneck_width",
            "output_width",
            "encoder_activation",
            "decoder_activation",
            "loss",
            "reduction",
            "trainable_parameters",
            "update",
        },
        "operation",
    )
    required_operation = {
        "kind": "two-to-one-to-two-autoencoder-step",
        "input_width": 2,
        "bottleneck_width": 1,
        "output_width": 2,
        "encoder_activation": "identity",
        "decoder_activation": "identity",
        "loss": "mean-squared-reconstruction-error",
        "reduction": "mean-over-output-coordinates",
        "trainable_parameters": [
            "encoder.weights",
            "encoder.bias",
            "decoder.weights",
            "decoder.bias",
        ],
        "update": "sgd",
    }
    if operation != required_operation:
        raise TwoNumberAutoencoderValidationError(
            "operation: unsupported NN16 contract"
        )

    _vector(document["input"], 2, "input")
    encoder = _object(document["encoder"], {"weights", "bias"}, "encoder")
    _vector(encoder["weights"], 2, "encoder.weights")
    _number(encoder["bias"], "encoder.bias")
    decoder = _object(document["decoder"], {"weights", "bias"}, "decoder")
    _vector(decoder["weights"], 2, "decoder.weights")
    _vector(decoder["bias"], 2, "decoder.bias")
    optimizer = _object(document["optimizer"], {"learning_rate"}, "optimizer")
    if _number(optimizer["learning_rate"], "optimizer.learning_rate") <= 0:
        raise TwoNumberAutoencoderValidationError(
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
        raise TwoNumberAutoencoderValidationError(
            "expected.gradient_check: analytical and numerical gradients disagree"
        )
    if actual["post_update"]["loss"] >= actual["forward"]["loss"]:
        raise TwoNumberAutoencoderValidationError(
            "expected.post_update.loss: canonical update must lower loss"
        )
    return actual


def validate_fixture_root(root: Path) -> int:
    labs = sorted((root / "labs").glob("*.json"))
    if not labs:
        raise TwoNumberAutoencoderValidationError(f"{root}: no lab documents found")
    for path in labs:
        validate_lab(load_json(path))
        print(f"validated {path.relative_to(REPO_ROOT)}")
    return len(labs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_fixture_root(args.root.resolve())
    print(f"validated {count} NN16 two-number autoencoder lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
