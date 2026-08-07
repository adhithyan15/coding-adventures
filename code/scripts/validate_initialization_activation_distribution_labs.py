#!/usr/bin/env python3
"""Validate and execute deterministic NN23 initialization-distribution labs."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT
    / "code"
    / "specs"
    / "fixtures"
    / "initialization-activation-distribution-v1"
)

INITIALIZERS = ["tiny", "xavier", "he", "large"]
ACTIVATIONS = ["tanh", "relu"]


class InitializationDistributionValidationError(ValueError):
    """Raised when an NN23 document or trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise InitializationDistributionValidationError(
                f"duplicate JSON key: {key}"
            )
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                InitializationDistributionValidationError(
                    f"non-finite JSON number: {item}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise InitializationDistributionValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise InitializationDistributionValidationError(
            "top-level JSON must be an object"
        )
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise InitializationDistributionValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise InitializationDistributionValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
    ):
        raise InitializationDistributionValidationError(
            f"{context}: expected finite number"
        )
    return float(value)


def _matrix(value: Any, context: str, minimum_rows: int = 1) -> list[list[float]]:
    if not isinstance(value, list) or len(value) < minimum_rows:
        raise InitializationDistributionValidationError(
            f"{context}: expected at least {minimum_rows} rows"
        )
    rows: list[list[float]] = []
    width: int | None = None
    for row_index, raw_row in enumerate(value):
        if not isinstance(raw_row, list) or not raw_row:
            raise InitializationDistributionValidationError(
                f"{context}[{row_index}]: expected non-empty row"
            )
        row = [
            _number(item, f"{context}[{row_index}][{column}]")
            for column, item in enumerate(raw_row)
        ]
        if width is None:
            width = len(row)
        elif len(row) != width:
            raise InitializationDistributionValidationError(
                f"{context}: expected rectangular matrix"
            )
        rows.append(row)
    return rows


def _scale(initializer: str, fan_in: int) -> float:
    if fan_in < 1:
        raise InitializationDistributionValidationError(
            "fan-in must be a positive integer"
        )
    if initializer == "tiny":
        return 0.1
    if initializer == "xavier":
        return math.sqrt(1 / fan_in)
    if initializer == "he":
        return math.sqrt(2 / fan_in)
    if initializer == "large":
        return 2.0
    raise InitializationDistributionValidationError(
        f"unsupported initializer: {initializer}"
    )


def _summary(matrix: list[list[float]], activation: str) -> dict[str, float]:
    values = [value for row in matrix for value in row]
    mean = sum(values) / len(values)
    variance = sum((value - mean) ** 2 for value in values) / len(values)
    return {
        "mean": mean,
        "variance": variance,
        "standard_deviation": math.sqrt(variance),
        "minimum": min(values),
        "maximum": max(values),
        "zero_fraction": sum(abs(value) < 1e-12 for value in values) / len(values),
        "saturated_fraction": (
            sum(abs(value) >= 0.95 for value in values) / len(values)
            if activation == "tanh"
            else 0.0
        ),
    }


def _trace(
    inputs: list[list[float]],
    templates: list[list[list[float]]],
    initializer: str,
    activation: str,
) -> list[dict[str, Any]]:
    current = [row[:] for row in inputs]
    layers: list[dict[str, Any]] = []
    for layer_index, template in enumerate(templates, start=1):
        fan_in = len(current[0])
        if len(template) != fan_in:
            raise InitializationDistributionValidationError(
                f"weight_templates[{layer_index - 1}]: must match fan-in {fan_in}"
            )
        scale = _scale(initializer, fan_in)
        weights = [[value * scale for value in row] for row in template]
        width = len(weights[0])
        preactivations = [
            [
                sum(
                    row[input_index] * weights[input_index][output]
                    for input_index in range(fan_in)
                )
                for output in range(width)
            ]
            for row in current
        ]
        activations = [
            [
                math.tanh(value) if activation == "tanh" else max(0.0, value)
                for value in row
            ]
            for row in preactivations
        ]
        layers.append(
            {
                "layer": layer_index,
                "scale": scale,
                "preactivations": preactivations,
                "activations": activations,
                "summary": _summary(activations, activation),
            }
        )
        current = activations
    return layers


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    inputs = _matrix(document["inputs"], "inputs", minimum_rows=2)
    raw_templates = document["weight_templates"]
    if not isinstance(raw_templates, list) or not raw_templates:
        raise InitializationDistributionValidationError(
            "weight_templates: expected at least one matrix"
        )
    templates = [
        _matrix(raw_template, f"weight_templates[{index}]")
        for index, raw_template in enumerate(raw_templates)
    ]
    canonical_layers = _trace(inputs, templates, "xavier", "tanh")
    comparison = []
    for initializer in INITIALIZERS:
        for activation in ACTIVATIONS:
            layers = _trace(inputs, templates, initializer, activation)
            comparison.append(
                {
                    "initializer": initializer,
                    "activation": activation,
                    "standard_deviations": [
                        layer["summary"]["standard_deviation"] for layer in layers
                    ],
                    "zero_fractions": [
                        layer["summary"]["zero_fraction"] for layer in layers
                    ],
                    "saturated_fractions": [
                        layer["summary"]["saturated_fraction"] for layer in layers
                    ],
                }
            )
    return {
        "canonical": {
            "initializer": "xavier",
            "activation": "tanh",
            "layers": canonical_layers,
        },
        "comparison": comparison,
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise InitializationDistributionValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        if abs(_number(actual, context) - float(expected)) > tolerance:
            raise InitializationDistributionValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise InitializationDistributionValidationError(f"{context}: mismatch")
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise InitializationDistributionValidationError(
                f"{context}: array length mismatch"
            )
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise InitializationDistributionValidationError(f"{context}: unsupported value")


def validate_document(document: dict[str, Any]) -> dict[str, Any]:
    root = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "inputs",
            "weight_templates",
            "initializers",
            "activations",
            "expected",
        },
        "document",
    )
    if root["schema_version"] != 1:
        raise InitializationDistributionValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise InitializationDistributionValidationError(
                f"{key}: expected non-empty string"
            )
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise InitializationDistributionValidationError(
            "absolute_tolerance must be positive"
        )
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or len(concepts) != len(set(concepts))
        or any(not isinstance(item, str) or not item for item in concepts)
    ):
        raise InitializationDistributionValidationError(
            "concepts must be unique non-empty strings"
        )
    operation = _object(
        root["operation"],
        {
            "kind",
            "bias",
            "variance",
            "zero_threshold",
            "tanh_saturation_threshold",
            "canonical_initializer",
            "canonical_activation",
        },
        "operation",
    )
    required = {
        "kind": "initialization-activation-distribution",
        "bias": "none",
        "variance": "population",
        "zero_threshold": 1e-12,
        "tanh_saturation_threshold": 0.95,
        "canonical_initializer": "xavier",
        "canonical_activation": "tanh",
    }
    if operation != required:
        raise InitializationDistributionValidationError(
            "operation does not match NN23 V1"
        )
    if root["initializers"] != INITIALIZERS or root["activations"] != ACTIVATIONS:
        raise InitializationDistributionValidationError(
            "initializer or activation order does not match NN23 V1"
        )
    actual = execute_lab(root)
    if not isinstance(root["expected"], dict):
        raise InitializationDistributionValidationError("expected must be object")
    _compare(actual, root["expected"], tolerance, "expected")
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    load_json(root / "schema.json")
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise InitializationDistributionValidationError("no labs found")
    for path in paths:
        validate_document(load_json(path))
        print(f"validated {path}")
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN23 initialization-distribution lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
