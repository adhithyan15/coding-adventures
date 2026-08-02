#!/usr/bin/env python3
"""Validate and execute deterministic NN26 tensor-broadcasting labs."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "tensor-broadcasting-v1"
)
CASE_IDS = [
    "outer-grid",
    "row-over-batch",
    "scalar-over-matrix",
    "incompatible-tail",
]
MAX_RANK = 4
MAX_DIMENSION = 8
MAX_VALUES = 64


class TensorBroadcastingValidationError(ValueError):
    """Raised when an NN26 document or computed trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise TensorBroadcastingValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                TensorBroadcastingValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise TensorBroadcastingValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise TensorBroadcastingValidationError("top-level JSON must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise TensorBroadcastingValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise TensorBroadcastingValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise TensorBroadcastingValidationError(f"{context}: expected finite number")
    try:
        number = float(value)
    except (OverflowError, ValueError) as error:
        raise TensorBroadcastingValidationError(
            f"{context}: expected finite number"
        ) from error
    if not math.isfinite(number):
        raise TensorBroadcastingValidationError(f"{context}: expected finite number")
    return number


def _shape(value: Any, context: str) -> list[int]:
    if not isinstance(value, list) or len(value) > MAX_RANK:
        raise TensorBroadcastingValidationError(
            f"{context}: expected at most {MAX_RANK} dimensions"
        )
    shape: list[int] = []
    for index, dimension in enumerate(value):
        if (
            isinstance(dimension, bool)
            or not isinstance(dimension, int)
            or dimension <= 0
            or dimension > MAX_DIMENSION
        ):
            raise TensorBroadcastingValidationError(
                f"{context}[{index}]: expected positive integer up to {MAX_DIMENSION}"
            )
        shape.append(dimension)
    return shape


def _element_count(shape: list[int]) -> int:
    count = math.prod(shape)
    if count > MAX_VALUES:
        raise TensorBroadcastingValidationError(
            f"tensor element count exceeds {MAX_VALUES}"
        )
    return count


def _tensor(value: Any, context: str) -> dict[str, Any]:
    tensor = _object(value, {"shape", "values"}, context)
    shape = _shape(tensor["shape"], f"{context}.shape")
    raw_values = tensor["values"]
    if not isinstance(raw_values, list):
        raise TensorBroadcastingValidationError(f"{context}.values: expected array")
    expected_count = _element_count(shape)
    if len(raw_values) != expected_count:
        raise TensorBroadcastingValidationError(
            f"{context}: value count {len(raw_values)} does not match shape count {expected_count}"
        )
    values = [
        _number(item, f"{context}.values[{index}]")
        for index, item in enumerate(raw_values)
    ]
    return {"shape": shape, "values": values}


def _strides(shape: list[int]) -> list[int]:
    result = [0] * len(shape)
    stride = 1
    for index in range(len(shape) - 1, -1, -1):
        result[index] = stride
        stride *= shape[index]
    return result


def _unravel(flat_index: int, shape: list[int]) -> list[int]:
    result = []
    for stride in _strides(shape):
        result.append(flat_index // stride)
        flat_index %= stride
    return result


def _flat_index(index: list[int], shape: list[int]) -> int:
    return sum(
        coordinate * stride for coordinate, stride in zip(index, _strides(shape))
    )


def _padded_shapes(
    left_shape: list[int], right_shape: list[int]
) -> tuple[list[int], list[int]]:
    rank = max(len(left_shape), len(right_shape))
    return (
        [1] * (rank - len(left_shape)) + left_shape,
        [1] * (rank - len(right_shape)) + right_shape,
    )


def _infer_shape(
    padded_left: list[int], padded_right: list[int]
) -> tuple[list[int] | None, int | None]:
    output = []
    for axis, (left_dimension, right_dimension) in enumerate(
        zip(padded_left, padded_right)
    ):
        if (
            left_dimension != right_dimension
            and left_dimension != 1
            and right_dimension != 1
        ):
            return None, axis
        output.append(max(left_dimension, right_dimension))
    return output, None


def _mappings(
    left: dict[str, Any],
    right: dict[str, Any],
    upstream: dict[str, Any],
    padded_left: list[int],
    padded_right: list[int],
    output_shape: list[int],
) -> list[dict[str, Any]]:
    rank = len(output_shape)
    left_leading = rank - len(left["shape"])
    right_leading = rank - len(right["shape"])
    result = []
    for output_flat_index in range(_element_count(output_shape)):
        output_index = _unravel(output_flat_index, output_shape)
        padded_left_index = [
            0 if padded_left[axis] == 1 else output_index[axis] for axis in range(rank)
        ]
        padded_right_index = [
            0 if padded_right[axis] == 1 else output_index[axis] for axis in range(rank)
        ]
        left_index = padded_left_index[left_leading:]
        right_index = padded_right_index[right_leading:]
        left_flat_index = _flat_index(left_index, left["shape"])
        right_flat_index = _flat_index(right_index, right["shape"])
        left_value = left["values"][left_flat_index]
        right_value = right["values"][right_flat_index]
        result.append(
            {
                "output_index": output_index,
                "output_flat_index": output_flat_index,
                "left_index": left_index,
                "left_flat_index": left_flat_index,
                "right_index": right_index,
                "right_flat_index": right_flat_index,
                "left_value": left_value,
                "right_value": right_value,
                "output_value": left_value + right_value,
                "upstream": upstream["values"][output_flat_index],
            }
        )
    return result


def _score(
    left_values: list[float],
    right_values: list[float],
    mappings: list[dict[str, Any]],
) -> float:
    return sum(
        mapping["upstream"]
        * (
            left_values[mapping["left_flat_index"]]
            + right_values[mapping["right_flat_index"]]
        )
        for mapping in mappings
    )


def execute_case(case: dict[str, Any], epsilon: float) -> dict[str, Any]:
    left, right = case["left"], case["right"]
    padded_left, padded_right = _padded_shapes(left["shape"], right["shape"])
    output_shape, mismatch_axis = _infer_shape(padded_left, padded_right)
    if output_shape is None:
        assert mismatch_axis is not None
        left_dimension = padded_left[mismatch_axis]
        right_dimension = padded_right[mismatch_axis]
        return {
            "compatible": False,
            "padded_left_shape": padded_left,
            "padded_right_shape": padded_right,
            "mismatch_axis": mismatch_axis,
            "left_dimension": left_dimension,
            "right_dimension": right_dimension,
            "error": (
                f"axis {mismatch_axis}: dimensions {left_dimension} and "
                f"{right_dimension} are incompatible"
            ),
        }

    upstream = case["upstream"]
    if upstream is None or upstream["shape"] != output_shape:
        raise TensorBroadcastingValidationError(
            f"{case['id']}: upstream shape must equal output shape {output_shape}"
        )
    mappings = _mappings(
        left,
        right,
        upstream,
        padded_left,
        padded_right,
        output_shape,
    )
    left_gradient = [0.0] * len(left["values"])
    right_gradient = [0.0] * len(right["values"])
    for mapping in mappings:
        left_gradient[mapping["left_flat_index"]] += mapping["upstream"]
        right_gradient[mapping["right_flat_index"]] += mapping["upstream"]

    finite_difference_left_gradient = []
    for index in range(len(left["values"])):
        positive, negative = list(left["values"]), list(left["values"])
        positive[index] += epsilon
        negative[index] -= epsilon
        finite_difference_left_gradient.append(
            (
                _score(positive, right["values"], mappings)
                - _score(negative, right["values"], mappings)
            )
            / (2 * epsilon)
        )
    finite_difference_right_gradient = []
    for index in range(len(right["values"])):
        positive, negative = list(right["values"]), list(right["values"])
        positive[index] += epsilon
        negative[index] -= epsilon
        finite_difference_right_gradient.append(
            (
                _score(left["values"], positive, mappings)
                - _score(left["values"], negative, mappings)
            )
            / (2 * epsilon)
        )
    errors = [
        abs(actual - numerical)
        for actual, numerical in zip(left_gradient, finite_difference_left_gradient)
    ] + [
        abs(actual - numerical)
        for actual, numerical in zip(right_gradient, finite_difference_right_gradient)
    ]
    return {
        "compatible": True,
        "padded_left_shape": padded_left,
        "padded_right_shape": padded_right,
        "output_shape": output_shape,
        "left_expanded_axes": [
            axis
            for axis, (dimension, output_dimension) in enumerate(
                zip(padded_left, output_shape)
            )
            if dimension == 1 and output_dimension > 1
        ],
        "right_expanded_axes": [
            axis
            for axis, (dimension, output_dimension) in enumerate(
                zip(padded_right, output_shape)
            )
            if dimension == 1 and output_dimension > 1
        ],
        "output_values": [mapping["output_value"] for mapping in mappings],
        "mappings": mappings,
        "left_gradient": left_gradient,
        "right_gradient": right_gradient,
        "finite_difference_left_gradient": finite_difference_left_gradient,
        "finite_difference_right_gradient": finite_difference_right_gradient,
        "max_gradient_absolute_error": max(errors, default=0.0),
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise TensorBroadcastingValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        expected_number = _number(expected, f"{context} expected value")
        if abs(_number(actual, context) - expected_number) > tolerance:
            raise TensorBroadcastingValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise TensorBroadcastingValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise TensorBroadcastingValidationError(f"{context}: array length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise TensorBroadcastingValidationError(
            f"{context}: unsupported expected value"
        )


def _validate_expected(expected: Any, context: str) -> dict[str, Any]:
    if not isinstance(expected, dict) or not isinstance(
        expected.get("compatible"), bool
    ):
        raise TensorBroadcastingValidationError(
            f"{context}: compatible must be a boolean"
        )
    if expected["compatible"]:
        keys = {
            "compatible",
            "padded_left_shape",
            "padded_right_shape",
            "output_shape",
            "left_expanded_axes",
            "right_expanded_axes",
            "output_values",
            "mappings",
            "left_gradient",
            "right_gradient",
            "finite_difference_left_gradient",
            "finite_difference_right_gradient",
            "max_gradient_absolute_error",
        }
        value = _object(expected, keys, context)
        if not isinstance(value["mappings"], list):
            raise TensorBroadcastingValidationError(
                f"{context}.mappings: expected array"
            )
        mapping_keys = {
            "output_index",
            "output_flat_index",
            "left_index",
            "left_flat_index",
            "right_index",
            "right_flat_index",
            "left_value",
            "right_value",
            "output_value",
            "upstream",
        }
        for index, mapping in enumerate(value["mappings"]):
            _object(mapping, mapping_keys, f"{context}.mappings[{index}]")
        return value
    return _object(
        expected,
        {
            "compatible",
            "padded_left_shape",
            "padded_right_shape",
            "mismatch_axis",
            "left_dimension",
            "right_dimension",
            "error",
        },
        context,
    )


def validate_document(document: dict[str, Any]) -> dict[str, Any]:
    root = _object(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "operation",
            "cases",
        },
        "document",
    )
    if root["schema_version"] != 1:
        raise TensorBroadcastingValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise TensorBroadcastingValidationError(f"{key}: expected non-empty string")
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance != 1e-8:
        raise TensorBroadcastingValidationError(
            "absolute_tolerance must be canonical 1e-8"
        )
    operation = _object(
        root["operation"],
        {
            "kind",
            "dimension_alignment",
            "mismatch_policy",
            "gradient_reduction",
            "storage_order",
            "finite_difference_epsilon",
        },
        "operation",
    )
    required_operation = {
        "kind": "elementwise-add-with-broadcasting",
        "dimension_alignment": "right",
        "mismatch_policy": "reject",
        "gradient_reduction": "sum-expanded-axes",
        "storage_order": "row-major",
        "finite_difference_epsilon": 1e-5,
    }
    if operation != required_operation:
        raise TensorBroadcastingValidationError(
            f"operation must equal {required_operation}"
        )
    cases = root["cases"]
    if not isinstance(cases, list) or len(cases) != len(CASE_IDS):
        raise TensorBroadcastingValidationError(
            f"cases must contain exactly {len(CASE_IDS)} entries"
        )
    actual_ids = [case.get("id") if isinstance(case, dict) else None for case in cases]
    if actual_ids != CASE_IDS:
        raise TensorBroadcastingValidationError(
            f"case ids must be ordered as {CASE_IDS}"
        )

    normalized_cases = []
    for index, raw_case in enumerate(cases):
        context = f"cases[{index}]"
        case = _object(
            raw_case,
            {"id", "title", "left", "right", "upstream", "expected"},
            context,
        )
        if not isinstance(case["title"], str) or not case["title"].strip():
            raise TensorBroadcastingValidationError(
                f"{context}.title: expected non-empty string"
            )
        normalized = {
            "id": case["id"],
            "title": case["title"],
            "left": _tensor(case["left"], f"{context}.left"),
            "right": _tensor(case["right"], f"{context}.right"),
            "upstream": None
            if case["upstream"] is None
            else _tensor(case["upstream"], f"{context}.upstream"),
            "expected": _validate_expected(case["expected"], f"{context}.expected"),
        }
        actual = execute_case(
            normalized, required_operation["finite_difference_epsilon"]
        )
        _compare(actual, normalized["expected"], tolerance, f"{context}.expected")
        if actual["compatible"] and actual["max_gradient_absolute_error"] > tolerance:
            raise TensorBroadcastingValidationError(
                f"{context}: finite-difference error exceeds tolerance"
            )
        normalized_cases.append(normalized)

    return {**root, "absolute_tolerance": tolerance, "cases": normalized_cases}


def validate_fixture_root(root: Path) -> int:
    if not (root / "schema.json").is_file():
        raise TensorBroadcastingValidationError(
            f"missing schema: {root / 'schema.json'}"
        )
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise TensorBroadcastingValidationError(
            f"no lab fixtures under {root / 'labs'}"
        )
    seen: set[str] = set()
    for path in paths:
        document = validate_document(load_json(path))
        if document["id"] in seen:
            raise TensorBroadcastingValidationError(
                f"duplicate lab id: {document['id']}"
            )
        seen.add(document["id"])
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "fixture_root",
        nargs="?",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )
    args = parser.parse_args()
    try:
        count = validate_fixture_root(args.fixture_root.resolve())
    except TensorBroadcastingValidationError as error:
        parser.error(str(error))
    print(f"validated {count} tensor broadcasting lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
