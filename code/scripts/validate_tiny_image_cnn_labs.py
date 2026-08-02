#!/usr/bin/env python3
"""Validate and execute the deterministic NN07 tiny-image CNN corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "tiny-image-cnn-v1"
)


class TinyImageCnnValidationError(ValueError):
    """Raised when an NN07 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TinyImageCnnValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                TinyImageCnnValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TinyImageCnnValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise TinyImageCnnValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise TinyImageCnnValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise TinyImageCnnValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TinyImageCnnValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise TinyImageCnnValidationError(f"{context}: expected a finite number")
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise TinyImageCnnValidationError(f"{context}: expected a positive number")
    return result


def _numbers(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise TinyImageCnnValidationError(
            f"{context}: expected a non-empty number array"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _matrix(value: Any, context: str) -> list[list[float]]:
    if not isinstance(value, list) or not value:
        raise TinyImageCnnValidationError(
            f"{context}: expected a non-empty matrix"
        )
    rows = [_numbers(row, f"{context}[{index}]") for index, row in enumerate(value)]
    width = len(rows[0])
    if any(len(row) != width for row in rows):
        raise TinyImageCnnValidationError(f"{context}: matrix must be rectangular")
    return rows


def _shape(matrix: list[list[float]]) -> tuple[int, int]:
    return len(matrix), len(matrix[0])


def correlate_channels(
    channels: list[list[list[float]]],
    filters: list[dict[str, Any]],
) -> dict[str, Any]:
    """Run valid stride-one 2D cross-correlation and preserve channel sums."""
    if not channels or not filters:
        raise TinyImageCnnValidationError("channels and filters must be non-empty")
    input_height, input_width = _shape(channels[0])
    if any(_shape(channel) != (input_height, input_width) for channel in channels):
        raise TinyImageCnnValidationError("all input channels must share one shape")

    all_contributions: list[list[list[list[float]]]] = []
    outputs: list[list[list[float]]] = []
    for filter_index, filter_value in enumerate(filters):
        kernels = filter_value["kernels"]
        if len(kernels) != len(channels):
            raise TinyImageCnnValidationError(
                f"filter {filter_index}: expected {len(channels)} kernels"
            )
        kernel_height, kernel_width = _shape(kernels[0])
        if any(_shape(kernel) != (kernel_height, kernel_width) for kernel in kernels):
            raise TinyImageCnnValidationError(
                f"filter {filter_index}: all kernels must share one shape"
            )
        if kernel_height > input_height or kernel_width > input_width:
            raise TinyImageCnnValidationError(
                f"filter {filter_index}: kernel must fit inside the image"
            )
        output_height = input_height - kernel_height + 1
        output_width = input_width - kernel_width + 1
        contribution_maps = [
            [[0.0 for _ in range(output_width)] for _ in range(output_height)]
            for _ in channels
        ]
        output_map = [
            [float(filter_value["bias"]) for _ in range(output_width)]
            for _ in range(output_height)
        ]
        for row in range(output_height):
            for column in range(output_width):
                for channel_index, (channel, kernel) in enumerate(
                    zip(channels, kernels)
                ):
                    contribution = sum(
                        channel[row + kernel_row][column + kernel_column]
                        * kernel[kernel_row][kernel_column]
                        for kernel_row in range(kernel_height)
                        for kernel_column in range(kernel_width)
                    )
                    contribution_maps[channel_index][row][column] = contribution
                    output_map[row][column] += contribution
        all_contributions.append(contribution_maps)
        outputs.append(output_map)
    return {"channel_contributions": all_contributions, "convolution": outputs}


def normalize_spatial(
    maps: list[list[list[float]]],
    epsilon: float,
    gamma: list[float],
    beta: list[float],
) -> dict[str, Any]:
    if len(gamma) != len(maps) or len(beta) != len(maps):
        raise TinyImageCnnValidationError(
            "normalization gamma and beta must match output channels"
        )
    means: list[float] = []
    variances: list[float] = []
    denominators: list[float] = []
    normalized_maps: list[list[list[float]]] = []
    for index, feature_map in enumerate(maps):
        values = [value for row in feature_map for value in row]
        mean = sum(values) / len(values)
        variance = sum((value - mean) ** 2 for value in values) / len(values)
        denominator = math.sqrt(variance + epsilon)
        means.append(mean)
        variances.append(variance)
        denominators.append(denominator)
        normalized_maps.append(
            [
                [
                    gamma[index] * (value - mean) / denominator + beta[index]
                    for value in row
                ]
                for row in feature_map
            ]
        )
    return {
        "means": means,
        "variances": variances,
        "denominators": denominators,
        "maps": normalized_maps,
    }


def relu_maps(maps: list[list[list[float]]]) -> list[list[list[float]]]:
    return [
        [[max(0.0, value) for value in row] for row in feature_map]
        for feature_map in maps
    ]


def max_pool_entire_maps(
    maps: list[list[list[float]]],
) -> dict[str, Any]:
    values: list[float] = []
    argmax: list[list[int]] = []
    for feature_map in maps:
        best_value = -math.inf
        best_position = [0, 0]
        for row_index, row in enumerate(feature_map):
            for column_index, value in enumerate(row):
                if value > best_value:
                    best_value = value
                    best_position = [row_index, column_index]
        values.append(best_value)
        argmax.append(best_position)
    return {"values": values, "argmax": argmax}


def trace_pipeline(lab: dict[str, Any]) -> dict[str, Any]:
    channels = [channel["values"] for channel in lab["input"]["channels"]]
    convolution = correlate_channels(channels, lab["filters"])
    normalization = normalize_spatial(
        convolution["convolution"],
        float(lab["normalization"]["epsilon"]),
        lab["normalization"]["gamma"],
        lab["normalization"]["beta"],
    )
    activation = relu_maps(normalization["maps"])
    return {
        **convolution,
        "normalization": normalization,
        "activation": activation,
        "pooling": max_pool_entire_maps(activation),
    }


def validate_structure(lab: dict[str, Any], source: str = "lab") -> None:
    _require_keys(
        lab,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "input",
            "filters",
            "normalization",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise TinyImageCnnValidationError(f"{source}.schema_version: expected 1")
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise TinyImageCnnValidationError(
                f"{source}.{field}: expected a non-empty string"
            )
    _positive_number(lab["absolute_tolerance"], f"{source}.absolute_tolerance")
    concepts = lab["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(concepts) != len(set(concepts))
    ):
        raise TinyImageCnnValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )

    operation = lab["operation"]
    if not isinstance(operation, dict):
        raise TinyImageCnnValidationError(f"{source}.operation: expected an object")
    _require_keys(
        operation,
        {"kind", "padding", "stride", "activation", "pooling"},
        f"{source}.operation",
    )
    pooling = operation["pooling"]
    if not isinstance(pooling, dict):
        raise TinyImageCnnValidationError(
            f"{source}.operation.pooling: expected an object"
        )
    _require_keys(
        pooling,
        {"kind", "window", "stride", "tie_break"},
        f"{source}.operation.pooling",
    )
    expected_operation = {
        "kind": "cross-correlation-2d",
        "padding": "valid",
        "stride": [1, 1],
        "activation": "relu",
    }
    if {key: operation[key] for key in expected_operation} != expected_operation:
        raise TinyImageCnnValidationError(
            f"{source}.operation: V1 requires valid stride-one 2D correlation and ReLU"
        )
    if pooling != {
        "kind": "max",
        "window": [2, 2],
        "stride": [2, 2],
        "tie_break": "first-row-major",
    }:
        raise TinyImageCnnValidationError(
            f"{source}.operation.pooling: V1 requires row-major 2 x 2 max pooling"
        )

    input_value = lab["input"]
    if not isinstance(input_value, dict):
        raise TinyImageCnnValidationError(f"{source}.input: expected an object")
    _require_keys(input_value, {"shape", "channels"}, f"{source}.input")
    if input_value["shape"] != [2, 3, 3]:
        raise TinyImageCnnValidationError(
            f"{source}.input.shape: V1 requires [2, 3, 3]"
        )
    channels = input_value["channels"]
    if not isinstance(channels, list) or len(channels) != 2:
        raise TinyImageCnnValidationError(
            f"{source}.input.channels: expected two channels"
        )
    parsed_channels: list[list[list[float]]] = []
    for index, channel in enumerate(channels):
        context = f"{source}.input.channels[{index}]"
        if not isinstance(channel, dict):
            raise TinyImageCnnValidationError(f"{context}: expected an object")
        _require_keys(channel, {"name", "values"}, context)
        if not isinstance(channel["name"], str) or not channel["name"]:
            raise TinyImageCnnValidationError(f"{context}.name: expected text")
        values = _matrix(channel["values"], f"{context}.values")
        if _shape(values) != (3, 3):
            raise TinyImageCnnValidationError(
                f"{context}.values: expected a 3 x 3 matrix"
            )
        parsed_channels.append(values)

    filters = lab["filters"]
    if not isinstance(filters, list) or len(filters) != 2:
        raise TinyImageCnnValidationError(f"{source}.filters: expected two filters")
    for filter_index, filter_value in enumerate(filters):
        context = f"{source}.filters[{filter_index}]"
        if not isinstance(filter_value, dict):
            raise TinyImageCnnValidationError(f"{context}: expected an object")
        _require_keys(filter_value, {"name", "kernels", "bias"}, context)
        if not isinstance(filter_value["name"], str) or not filter_value["name"]:
            raise TinyImageCnnValidationError(f"{context}.name: expected text")
        _number(filter_value["bias"], f"{context}.bias")
        kernels = filter_value["kernels"]
        if not isinstance(kernels, list) or len(kernels) != 2:
            raise TinyImageCnnValidationError(
                f"{context}.kernels: expected one kernel per input channel"
            )
        filter_value["kernels"] = [
            _matrix(kernel, f"{context}.kernels[{index}]")
            for index, kernel in enumerate(kernels)
        ]
        if any(_shape(kernel) != (2, 2) for kernel in filter_value["kernels"]):
            raise TinyImageCnnValidationError(
                f"{context}.kernels: V1 requires 2 x 2 kernels"
            )

    normalization = lab["normalization"]
    if not isinstance(normalization, dict):
        raise TinyImageCnnValidationError(
            f"{source}.normalization: expected an object"
        )
    _require_keys(
        normalization,
        {"kind", "epsilon", "gamma", "beta"},
        f"{source}.normalization",
    )
    if normalization["kind"] != "spatial-per-output-channel":
        raise TinyImageCnnValidationError(
            f"{source}.normalization.kind: unsupported normalization"
        )
    _positive_number(normalization["epsilon"], f"{source}.normalization.epsilon")
    for field in ("gamma", "beta"):
        values = _numbers(normalization[field], f"{source}.normalization.{field}")
        if len(values) != 2:
            raise TinyImageCnnValidationError(
                f"{source}.normalization.{field}: expected two values"
            )

    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise TinyImageCnnValidationError(f"{source}.expected: expected an object")
    _require_keys(
        expected,
        {"channel_contributions", "convolution", "normalization", "activation", "pooling"},
        f"{source}.expected",
    )
    expected_normalization = expected["normalization"]
    expected_pooling = expected["pooling"]
    if not isinstance(expected_normalization, dict) or not isinstance(expected_pooling, dict):
        raise TinyImageCnnValidationError(
            f"{source}.expected: normalization and pooling must be objects"
        )
    _require_keys(
        expected_normalization,
        {"means", "variances", "denominators", "maps"},
        f"{source}.expected.normalization",
    )
    _require_keys(
        expected_pooling,
        {"values", "argmax"},
        f"{source}.expected.pooling",
    )


def _compare_nested(
    actual: Any, expected: Any, tolerance: float, context: str
) -> None:
    if isinstance(actual, list):
        if not isinstance(expected, list) or len(actual) != len(expected):
            raise TinyImageCnnValidationError(
                f"{context}: expected a list of length {len(actual)}"
            )
        for index, actual_value in enumerate(actual):
            _compare_nested(
                actual_value, expected[index], tolerance, f"{context}[{index}]"
            )
        return
    actual_number = _number(actual, context)
    expected_number = _number(expected, context)
    if abs(actual_number - expected_number) > tolerance:
        raise TinyImageCnnValidationError(
            f"{context}: expected {expected_number!r}, got {actual_number!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    actual = trace_pipeline(lab)
    expected = lab["expected"]
    tolerance = float(lab["absolute_tolerance"])
    for field in ("channel_contributions", "convolution", "activation"):
        _compare_nested(
            actual[field], expected[field], tolerance, f"{source}.expected.{field}"
        )
    for field in ("means", "variances", "denominators", "maps"):
        _compare_nested(
            actual["normalization"][field],
            expected["normalization"][field],
            tolerance,
            f"{source}.expected.normalization.{field}",
        )
    _compare_nested(
        actual["pooling"]["values"],
        expected["pooling"]["values"],
        tolerance,
        f"{source}.expected.pooling.values",
    )
    if actual["pooling"]["argmax"] != expected["pooling"]["argmax"]:
        raise TinyImageCnnValidationError(
            f"{source}.expected.pooling.argmax: expected "
            f"{expected['pooling']['argmax']!r}, got {actual['pooling']['argmax']!r}"
        )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise TinyImageCnnValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise TinyImageCnnValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise TinyImageCnnValidationError(
                f"{path}: duplicate lab id {lab['id']!r}"
            )
        ids.add(lab["id"])
    return lab_paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    try:
        paths = validate_corpus(args.fixture_root)
    except TinyImageCnnValidationError as error:
        parser.exit(1, f"tiny image CNN corpus invalid: {error}\n")
    print(f"validated {len(paths)} tiny image CNN labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
