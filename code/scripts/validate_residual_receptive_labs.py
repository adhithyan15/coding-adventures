#!/usr/bin/env python3
"""Validate and execute the deterministic NN08 residual/receptive corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "residual-receptive-v1"
)


class ResidualReceptiveValidationError(ValueError):
    """Raised when an NN08 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ResidualReceptiveValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ResidualReceptiveValidationError(
                    f"non-finite JSON number: {value}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ResidualReceptiveValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise ResidualReceptiveValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise ResidualReceptiveValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise ResidualReceptiveValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ResidualReceptiveValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise ResidualReceptiveValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise ResidualReceptiveValidationError(
            f"{context}: expected a positive number"
        )
    return result


def _numbers(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ResidualReceptiveValidationError(
            f"{context}: expected a non-empty number array"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _indices(value: Any, context: str) -> list[int]:
    if (
        not isinstance(value, list)
        or not value
        or any(isinstance(item, bool) or not isinstance(item, int) or item < 0 for item in value)
    ):
        raise ResidualReceptiveValidationError(
            f"{context}: expected non-negative integer indices"
        )
    return value


def same_correlation(signal: list[float], kernel: list[float]) -> list[float]:
    if not signal or not kernel or len(kernel) % 2 == 0:
        raise ResidualReceptiveValidationError(
            "same correlation requires a non-empty signal and odd kernel"
        )
    radius = len(kernel) // 2
    return [
        sum(
            (signal[index + offset] if 0 <= index + offset < len(signal) else 0.0)
            * kernel[offset + radius]
            for offset in range(-radius, radius + 1)
        )
        for index in range(len(signal))
    ]


def trace_residual(input_values: list[float], kernels: list[list[float]]) -> dict[str, Any]:
    if len(kernels) != 2 or any(kernel != [1.0, 1.0, 1.0] for kernel in kernels):
        raise ResidualReceptiveValidationError(
            "V1 requires two [1, 1, 1] kernels"
        )
    hidden = same_correlation(input_values, kernels[0])
    main = same_correlation(hidden, kernels[1])
    skip = list(input_values)
    residual_sum = [main_value + skip_value for main_value, skip_value in zip(main, skip)]
    output = [max(0.0, value) for value in residual_sum]
    traces: list[dict[str, Any]] = []
    for output_index in range(len(input_values)):
        hidden_indices = [
            index
            for index in range(output_index - 1, output_index + 2)
            if 0 <= index < len(input_values)
        ]
        path_counts = [0 for _ in input_values]
        hidden_paths: list[dict[str, Any]] = []
        for hidden_index in hidden_indices:
            input_indices = [
                index
                for index in range(hidden_index - 1, hidden_index + 2)
                if 0 <= index < len(input_values)
            ]
            for input_index in input_indices:
                path_counts[input_index] += 1
            hidden_paths.append(
                {
                    "hidden_index": hidden_index,
                    "input_indices": input_indices,
                    "input_values": [input_values[index] for index in input_indices],
                    "subtotal": hidden[hidden_index],
                }
            )
        traces.append(
            {
                "output_index": output_index,
                "hidden_indices": hidden_indices,
                "hidden_values": [hidden[index] for index in hidden_indices],
                "hidden_paths": hidden_paths,
                "input_path_counts": path_counts,
                "input_contributions": [
                    input_values[index] * path_counts[index]
                    for index in range(len(input_values))
                ],
                "receptive_field_indices": [
                    index for index, count in enumerate(path_counts) if count > 0
                ],
                "main_output": main[output_index],
                "skip_contribution": skip[output_index],
                "residual_sum": residual_sum[output_index],
                "output": output[output_index],
            }
        )
    return {
        "hidden": hidden,
        "main": main,
        "skip": skip,
        "residual_sum": residual_sum,
        "output": output,
        "traces": traces,
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
            "kernels",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise ResidualReceptiveValidationError(
            f"{source}.schema_version: expected 1"
        )
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise ResidualReceptiveValidationError(
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
        raise ResidualReceptiveValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )

    operation = lab["operation"]
    expected_operation = {
        "kind": "residual-correlation-1d",
        "padding": "same-zero",
        "stride": 1,
        "layers": 2,
        "skip": "identity",
        "activation": "relu",
    }
    if not isinstance(operation, dict):
        raise ResidualReceptiveValidationError(
            f"{source}.operation: expected an object"
        )
    _require_keys(operation, set(expected_operation), f"{source}.operation")
    if operation != expected_operation:
        raise ResidualReceptiveValidationError(
            f"{source}.operation: unsupported V1 operation"
        )

    input_values = _numbers(lab["input"], f"{source}.input")
    if len(input_values) != 5:
        raise ResidualReceptiveValidationError(
            f"{source}.input: V1 requires five values"
        )
    kernels_value = lab["kernels"]
    if not isinstance(kernels_value, list) or len(kernels_value) != 2:
        raise ResidualReceptiveValidationError(
            f"{source}.kernels: expected two kernels"
        )
    kernels = [
        _numbers(kernel, f"{source}.kernels[{index}]")
        for index, kernel in enumerate(kernels_value)
    ]
    if any(kernel != [1.0, 1.0, 1.0] for kernel in kernels):
        raise ResidualReceptiveValidationError(
            f"{source}.kernels: V1 requires two [1, 1, 1] kernels"
        )

    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise ResidualReceptiveValidationError(
            f"{source}.expected: expected an object"
        )
    _require_keys(
        expected,
        {"hidden", "main", "skip", "residual_sum", "output", "traces"},
        f"{source}.expected",
    )
    for field in ("hidden", "main", "skip", "residual_sum", "output"):
        if len(_numbers(expected[field], f"{source}.expected.{field}")) != 5:
            raise ResidualReceptiveValidationError(
                f"{source}.expected.{field}: expected five values"
            )
    traces = expected["traces"]
    if not isinstance(traces, list) or len(traces) != 5:
        raise ResidualReceptiveValidationError(
            f"{source}.expected.traces: expected five traces"
        )
    trace_keys = {
        "output_index",
        "hidden_indices",
        "hidden_values",
        "hidden_paths",
        "input_path_counts",
        "input_contributions",
        "receptive_field_indices",
        "main_output",
        "skip_contribution",
        "residual_sum",
        "output",
    }
    for index, trace in enumerate(traces):
        context = f"{source}.expected.traces[{index}]"
        if not isinstance(trace, dict):
            raise ResidualReceptiveValidationError(f"{context}: expected an object")
        _require_keys(trace, trace_keys, context)
        if trace["output_index"] != index:
            raise ResidualReceptiveValidationError(
                f"{context}.output_index: expected {index}"
            )
        hidden_indices = _indices(trace["hidden_indices"], f"{context}.hidden_indices")
        if len(_numbers(trace["hidden_values"], f"{context}.hidden_values")) != len(hidden_indices):
            raise ResidualReceptiveValidationError(
                f"{context}.hidden_values: must match hidden indices"
            )
        paths = trace["hidden_paths"]
        if not isinstance(paths, list) or len(paths) != len(hidden_indices):
            raise ResidualReceptiveValidationError(
                f"{context}.hidden_paths: must match hidden indices"
            )
        for path_index, path in enumerate(paths):
            path_context = f"{context}.hidden_paths[{path_index}]"
            if not isinstance(path, dict):
                raise ResidualReceptiveValidationError(
                    f"{path_context}: expected an object"
                )
            _require_keys(
                path,
                {"hidden_index", "input_indices", "input_values", "subtotal"},
                path_context,
            )
            input_indices = _indices(
                path["input_indices"], f"{path_context}.input_indices"
            )
            if len(_numbers(path["input_values"], f"{path_context}.input_values")) != len(input_indices):
                raise ResidualReceptiveValidationError(
                    f"{path_context}.input_values: must match input indices"
                )
            _number(path["subtotal"], f"{path_context}.subtotal")
        for field in ("input_path_counts", "input_contributions"):
            if len(_numbers(trace[field], f"{context}.{field}")) != 5:
                raise ResidualReceptiveValidationError(
                    f"{context}.{field}: expected five values"
                )
        _indices(
            trace["receptive_field_indices"],
            f"{context}.receptive_field_indices",
        )
        for field in ("main_output", "skip_contribution", "residual_sum", "output"):
            _number(trace[field], f"{context}.{field}")


def _compare_nested(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(actual, dict):
        if not isinstance(expected, dict) or actual.keys() != expected.keys():
            raise ResidualReceptiveValidationError(
                f"{context}: object keys do not match"
            )
        for key, actual_value in actual.items():
            _compare_nested(actual_value, expected[key], tolerance, f"{context}.{key}")
        return
    if isinstance(actual, list):
        if not isinstance(expected, list) or len(actual) != len(expected):
            raise ResidualReceptiveValidationError(
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
        raise ResidualReceptiveValidationError(
            f"{context}: expected {expected_number!r}, got {actual_number!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    actual = trace_residual(
        [float(value) for value in lab["input"]],
        [[float(value) for value in kernel] for kernel in lab["kernels"]],
    )
    _compare_nested(
        actual,
        lab["expected"],
        float(lab["absolute_tolerance"]),
        f"{source}.expected",
    )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise ResidualReceptiveValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise ResidualReceptiveValidationError(
            f"{fixture_root}: no lab fixtures found"
        )
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise ResidualReceptiveValidationError(
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
    except ResidualReceptiveValidationError as error:
        parser.exit(1, f"residual/receptive corpus invalid: {error}\n")
    print(f"validated {len(paths)} residual/receptive labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
