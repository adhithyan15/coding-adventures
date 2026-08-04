#!/usr/bin/env python3
"""Validate and execute deterministic NN25 training-stabilizer labs."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "training-stabilizers-v1"
)
ROUTE_IDS = ["plain", "normalization", "dropout", "residual"]


class TrainingStabilizerValidationError(ValueError):
    """Raised when an NN25 document or trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TrainingStabilizerValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                TrainingStabilizerValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TrainingStabilizerValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise TrainingStabilizerValidationError("top-level JSON must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TrainingStabilizerValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise TrainingStabilizerValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
    ):
        raise TrainingStabilizerValidationError(f"{context}: expected finite number")
    return float(value)


def _vector(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or len(value) != 4:
        raise TrainingStabilizerValidationError(f"{context}: expected four values")
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _dot(left: list[float], right: list[float]) -> float:
    return sum(a * b for a, b in zip(left, right))


def _normalization(
    values: list[float], upstream: list[float], epsilon: float
) -> dict[str, Any]:
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / len(values)
    standard_deviation = math.sqrt(variance + epsilon)
    if standard_deviation == 0:
        raise TrainingStabilizerValidationError(
            "normalization variance must be positive"
        )
    normalized = [value / standard_deviation for value in centered]
    return {
        "mean": mean,
        "centered": centered,
        "variance": variance,
        "standard_deviation": standard_deviation,
        "normalized": normalized,
        "upstream_sum": sum(upstream),
        "upstream_dot_normalized": _dot(upstream, normalized),
    }


def _route_output(
    route_id: str,
    inputs: list[float],
    branch_weight: float,
    mask: list[float],
    keep_probability: float,
    normalization_epsilon: float,
) -> list[float]:
    branch = [branch_weight * value for value in inputs]
    if route_id == "plain":
        return branch
    if route_id == "normalization":
        return _normalization(branch, [0.0] * 4, normalization_epsilon)["normalized"]
    if route_id == "dropout":
        return [
            value * mask[index] / keep_probability for index, value in enumerate(branch)
        ]
    if route_id == "residual":
        return [value + inputs[index] for index, value in enumerate(branch)]
    raise TrainingStabilizerValidationError(f"unknown route: {route_id}")


def _trace_route(
    route_id: str,
    inputs: list[float],
    branch_weight: float,
    upstream: list[float],
    mask: list[float],
    keep_probability: float,
    normalization_epsilon: float,
    finite_difference_epsilon: float,
    normalization: dict[str, Any],
) -> dict[str, Any]:
    output = _route_output(
        route_id,
        inputs,
        branch_weight,
        mask,
        keep_probability,
        normalization_epsilon,
    )
    if route_id == "normalization":
        count = len(inputs)
        denominator = count * normalization["standard_deviation"]
        branch_gradient = [
            (
                count * upstream[index]
                - normalization["upstream_sum"]
                - normalization["normalized"][index]
                * normalization["upstream_dot_normalized"]
            )
            / denominator
            for index in range(count)
        ]
    elif route_id == "dropout":
        branch_gradient = [
            upstream[index] * mask[index] / keep_probability
            for index in range(len(inputs))
        ]
    else:
        branch_gradient = list(upstream)
    skip_gradient = list(upstream) if route_id == "residual" else [0.0] * len(inputs)
    input_gradient = [
        branch_weight * branch_gradient[index] + skip_gradient[index]
        for index in range(len(inputs))
    ]
    weight_gradient = _dot(branch_gradient, inputs)
    score = _dot(upstream, output)

    finite_difference_input_gradient = []
    for index in range(len(inputs)):
        positive = list(inputs)
        negative = list(inputs)
        positive[index] += finite_difference_epsilon
        negative[index] -= finite_difference_epsilon
        positive_score = _dot(
            upstream,
            _route_output(
                route_id,
                positive,
                branch_weight,
                mask,
                keep_probability,
                normalization_epsilon,
            ),
        )
        negative_score = _dot(
            upstream,
            _route_output(
                route_id,
                negative,
                branch_weight,
                mask,
                keep_probability,
                normalization_epsilon,
            ),
        )
        finite_difference_input_gradient.append(
            (positive_score - negative_score) / (2 * finite_difference_epsilon)
        )
    positive_weight_score = _dot(
        upstream,
        _route_output(
            route_id,
            inputs,
            branch_weight + finite_difference_epsilon,
            mask,
            keep_probability,
            normalization_epsilon,
        ),
    )
    negative_weight_score = _dot(
        upstream,
        _route_output(
            route_id,
            inputs,
            branch_weight - finite_difference_epsilon,
            mask,
            keep_probability,
            normalization_epsilon,
        ),
    )
    finite_difference_weight_gradient = (
        positive_weight_score - negative_weight_score
    ) / (2 * finite_difference_epsilon)
    return {
        "id": route_id,
        "output": output,
        "score": score,
        "branch_gradient": branch_gradient,
        "skip_gradient": skip_gradient,
        "input_gradient": input_gradient,
        "weight_gradient": weight_gradient,
        "finite_difference_input_gradient": finite_difference_input_gradient,
        "finite_difference_weight_gradient": finite_difference_weight_gradient,
        "input_gradient_absolute_error": [
            abs(actual - numerical)
            for actual, numerical in zip(
                input_gradient, finite_difference_input_gradient
            )
        ],
        "weight_gradient_absolute_error": abs(
            weight_gradient - finite_difference_weight_gradient
        ),
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    operation = document["operation"]
    inputs = _vector(document["input"], "input")
    branch_weight = _number(document["branch_weight"], "branch_weight")
    upstream = _vector(document["upstream_gradient"], "upstream_gradient")
    raw_mask = document["dropout_mask"]
    if (
        not isinstance(raw_mask, list)
        or len(raw_mask) != 4
        or any(isinstance(item, bool) or item not in {0, 1} for item in raw_mask)
    ):
        raise TrainingStabilizerValidationError(
            "dropout_mask must contain four binary values"
        )
    mask = [float(item) for item in raw_mask]
    keep_probability = _number(
        operation["keep_probability"], "operation.keep_probability"
    )
    normalization_epsilon = _number(
        operation["normalization_epsilon"], "operation.normalization_epsilon"
    )
    finite_difference_epsilon = _number(
        operation["finite_difference_epsilon"], "operation.finite_difference_epsilon"
    )
    if not 0 < keep_probability <= 1:
        raise TrainingStabilizerValidationError("keep_probability must be in (0, 1]")
    if normalization_epsilon < 0 or finite_difference_epsilon <= 0:
        raise TrainingStabilizerValidationError(
            "epsilon values must be nonnegative and finite-difference epsilon positive"
        )
    branch = [branch_weight * value for value in inputs]
    normalization = _normalization(branch, upstream, normalization_epsilon)
    scaled_mask = [value / keep_probability for value in mask]
    dropout = {
        "scaled_mask": scaled_mask,
        "evaluation_output": list(branch),
        "training_expectation": list(branch),
    }
    routes = [
        _trace_route(
            route_id,
            inputs,
            branch_weight,
            upstream,
            mask,
            keep_probability,
            normalization_epsilon,
            finite_difference_epsilon,
            normalization,
        )
        for route_id in ROUTE_IDS
    ]
    return {
        "branch": branch,
        "normalization": normalization,
        "dropout": dropout,
        "routes": routes,
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise TrainingStabilizerValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        if abs(_number(actual, context) - float(expected)) > tolerance:
            raise TrainingStabilizerValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise TrainingStabilizerValidationError(f"{context}: mismatch")
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise TrainingStabilizerValidationError(f"{context}: array length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise TrainingStabilizerValidationError(f"{context}: unsupported value")


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
            "input",
            "branch_weight",
            "upstream_gradient",
            "dropout_mask",
            "expected",
        },
        "document",
    )
    if root["schema_version"] != 1:
        raise TrainingStabilizerValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise TrainingStabilizerValidationError(f"{key}: expected non-empty string")
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise TrainingStabilizerValidationError("absolute_tolerance must be positive")
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or len(concepts) != len(set(concepts))
        or any(not isinstance(item, str) or not item for item in concepts)
    ):
        raise TrainingStabilizerValidationError(
            "concepts must be unique non-empty strings"
        )
    operation = _object(
        root["operation"],
        {
            "kind",
            "normalization",
            "normalization_epsilon",
            "dropout",
            "keep_probability",
            "finite_difference_epsilon",
        },
        "operation",
    )
    required = {
        "kind": "vector-training-stabilizers",
        "normalization": "population-layer-normalization",
        "normalization_epsilon": 0,
        "dropout": "inverted-training-dropout",
        "keep_probability": 0.5,
        "finite_difference_epsilon": 1e-6,
    }
    if operation != required:
        raise TrainingStabilizerValidationError("operation does not match NN25 V1")
    expected = root["expected"]
    if not isinstance(expected, dict):
        raise TrainingStabilizerValidationError("expected must be object")
    raw_routes = expected.get("routes")
    if (
        not isinstance(raw_routes, list)
        or [item.get("id") for item in raw_routes if isinstance(item, dict)]
        != ROUTE_IDS
    ):
        raise TrainingStabilizerValidationError(
            "expected route order does not match NN25 V1"
        )
    actual = execute_lab(root)
    _compare(actual, expected, tolerance, "expected")
    for route in actual["routes"]:
        if (
            max(route["input_gradient_absolute_error"]) > 1e-8
            or route["weight_gradient_absolute_error"] > 1e-8
        ):
            raise TrainingStabilizerValidationError(
                f"{route['id']}: finite-difference gradient check failed"
            )
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    load_json(root / "schema.json")
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise TrainingStabilizerValidationError("no labs found")
    for path in paths:
        validate_document(load_json(path))
        print(f"validated {path}")
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN25 training-stabilizer lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
