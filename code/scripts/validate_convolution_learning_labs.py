#!/usr/bin/env python3
"""Validate and execute the deterministic NN05 convolution corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "convolution-learning-v1"
)


class ConvolutionLabValidationError(ValueError):
    """Raised when a convolution lab is structurally or numerically invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ConvolutionLabValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ConvolutionLabValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ConvolutionLabValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise ConvolutionLabValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise ConvolutionLabValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise ConvolutionLabValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ConvolutionLabValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise ConvolutionLabValidationError(f"{context}: expected a finite number")
    return result


def _numbers(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ConvolutionLabValidationError(
            f"{context}: expected a non-empty number array"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def trace_valid_correlation(
    signal: list[float], kernel: list[float]
) -> list[dict[str, Any]]:
    """Return every V1 output and multiply-accumulate intermediate."""
    if not signal or not kernel or len(kernel) > len(signal):
        raise ConvolutionLabValidationError(
            "signal and kernel must be non-empty, with kernel no longer than signal"
        )

    positions: list[dict[str, Any]] = []
    for start in range(len(signal) - len(kernel) + 1):
        window = signal[start : start + len(kernel)]
        products = [value * weight for value, weight in zip(window, kernel)]
        accumulator = [0.0]
        for product in products:
            accumulator.append(accumulator[-1] + product)
        positions.append(
            {
                "output_index": start,
                "start_index": start,
                "window": window,
                "products": products,
                "accumulator": accumulator,
                "output": accumulator[-1],
            }
        )
    return positions


def validate_structure(lab: dict[str, Any], source: str = "lab") -> None:
    _require_keys(
        lab,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "concepts",
            "operation",
            "signal",
            "kernel",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise ConvolutionLabValidationError(f"{source}.schema_version: expected 1")
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise ConvolutionLabValidationError(
                f"{source}.{field}: expected a non-empty string"
            )
    concepts = lab["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(concepts) != len(set(concepts))
    ):
        raise ConvolutionLabValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )

    operation = lab["operation"]
    if not isinstance(operation, dict):
        raise ConvolutionLabValidationError(f"{source}.operation: expected an object")
    _require_keys(operation, {"kind", "padding", "stride"}, f"{source}.operation")
    if operation != {
        "kind": "cross-correlation-1d",
        "padding": "valid",
        "stride": 1,
    }:
        raise ConvolutionLabValidationError(
            f"{source}.operation: V1 requires valid stride-one 1D cross-correlation"
        )

    signal = _numbers(lab["signal"], f"{source}.signal")
    kernel = _numbers(lab["kernel"], f"{source}.kernel")
    if len(kernel) > len(signal):
        raise ConvolutionLabValidationError(
            f"{source}.kernel: must be no longer than signal"
        )

    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise ConvolutionLabValidationError(f"{source}.expected: expected an object")
    _require_keys(
        expected, {"absolute_tolerance", "outputs", "positions"}, f"{source}.expected"
    )
    tolerance = _number(
        expected["absolute_tolerance"], f"{source}.expected.absolute_tolerance"
    )
    if tolerance <= 0:
        raise ConvolutionLabValidationError(
            f"{source}.expected.absolute_tolerance: expected a positive number"
        )
    output_count = len(signal) - len(kernel) + 1
    if len(_numbers(expected["outputs"], f"{source}.expected.outputs")) != output_count:
        raise ConvolutionLabValidationError(
            f"{source}.expected.outputs: expected {output_count} values"
        )
    positions = expected["positions"]
    if not isinstance(positions, list) or len(positions) != output_count:
        raise ConvolutionLabValidationError(
            f"{source}.expected.positions: expected {output_count} positions"
        )
    for index, position in enumerate(positions):
        context = f"{source}.expected.positions[{index}]"
        if not isinstance(position, dict):
            raise ConvolutionLabValidationError(f"{context}: expected an object")
        _require_keys(
            position,
            {
                "output_index",
                "start_index",
                "window",
                "products",
                "accumulator",
                "output",
            },
            context,
        )
        if position["output_index"] != index or position["start_index"] != index:
            raise ConvolutionLabValidationError(
                f"{context}: indices must equal the zero-based position"
            )
        for field, length in (
            ("window", len(kernel)),
            ("products", len(kernel)),
            ("accumulator", len(kernel) + 1),
        ):
            if len(_numbers(position[field], f"{context}.{field}")) != length:
                raise ConvolutionLabValidationError(
                    f"{context}.{field}: expected {length} values"
                )
        _number(position["output"], f"{context}.output")


def _compare(actual: float, expected: Any, tolerance: float, context: str) -> None:
    expected_number = _number(expected, context)
    if abs(actual - expected_number) > tolerance:
        raise ConvolutionLabValidationError(
            f"{context}: expected {expected_number!r}, got {actual!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    signal = _numbers(lab["signal"], f"{source}.signal")
    kernel = _numbers(lab["kernel"], f"{source}.kernel")
    actual_positions = trace_valid_correlation(signal, kernel)
    expected = lab["expected"]
    tolerance = float(expected["absolute_tolerance"])

    for index, actual in enumerate(actual_positions):
        expected_position = expected["positions"][index]
        for field in ("window", "products", "accumulator"):
            for item_index, actual_value in enumerate(actual[field]):
                _compare(
                    actual_value,
                    expected_position[field][item_index],
                    tolerance,
                    f"{source}.expected.positions[{index}].{field}[{item_index}]",
                )
        _compare(
            actual["output"],
            expected_position["output"],
            tolerance,
            f"{source}.expected.positions[{index}].output",
        )
        _compare(
            actual["output"],
            expected["outputs"][index],
            tolerance,
            f"{source}.expected.outputs[{index}]",
        )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise ConvolutionLabValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise ConvolutionLabValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise ConvolutionLabValidationError(
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
    except ConvolutionLabValidationError as error:
        parser.exit(1, f"convolution learning corpus invalid: {error}\n")
    print(f"validated {len(paths)} convolution learning labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
