#!/usr/bin/env python3
"""Validate and execute the deterministic NN06 convolution-training corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "convolution-training-v1"
)


class ConvolutionTrainingValidationError(ValueError):
    """Raised when a convolution-training lab is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ConvolutionTrainingValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ConvolutionTrainingValidationError(
                    f"non-finite JSON number: {value}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise ConvolutionTrainingValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise ConvolutionTrainingValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise ConvolutionTrainingValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise ConvolutionTrainingValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ConvolutionTrainingValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise ConvolutionTrainingValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise ConvolutionTrainingValidationError(
            f"{context}: expected a positive number"
        )
    return result


def _numbers(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ConvolutionTrainingValidationError(
            f"{context}: expected a non-empty number array"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def valid_correlation(signal: list[float], kernel: list[float]) -> list[float]:
    if not signal or not kernel or len(kernel) > len(signal):
        raise ConvolutionTrainingValidationError(
            "signal and kernel must be non-empty, with kernel no longer than signal"
        )
    return [
        sum(signal[start + offset] * weight for offset, weight in enumerate(kernel))
        for start in range(len(signal) - len(kernel) + 1)
    ]


def mean_squared_error(outputs: list[float], targets: list[float]) -> float:
    if len(outputs) != len(targets) or not outputs:
        raise ConvolutionTrainingValidationError(
            "outputs and targets must have the same non-zero length"
        )
    return sum(
        (output - target) ** 2 for output, target in zip(outputs, targets)
    ) / len(outputs)


def trace_training(
    signal: list[float], kernel: list[float], targets: list[float]
) -> dict[str, Any]:
    outputs = valid_correlation(signal, kernel)
    if len(outputs) != len(targets):
        raise ConvolutionTrainingValidationError(
            f"expected {len(outputs)} targets, got {len(targets)}"
        )
    errors = [output - target for output, target in zip(outputs, targets)]
    scale = 2.0 / len(outputs)
    output_gradients = [scale * error for error in errors]
    contributions: list[dict[str, Any]] = []
    kernel_gradient = [0.0 for _ in kernel]
    for output_index, output_gradient in enumerate(output_gradients):
        window = signal[output_index : output_index + len(kernel)]
        contribution = [output_gradient * value for value in window]
        for kernel_index, value in enumerate(contribution):
            kernel_gradient[kernel_index] += value
        contributions.append(
            {
                "output_index": output_index,
                "window": window,
                "output_gradient": output_gradient,
                "kernel_gradient": contribution,
            }
        )
    return {
        "outputs": outputs,
        "errors": errors,
        "loss": mean_squared_error(outputs, targets),
        "output_gradients": output_gradients,
        "contributions": contributions,
        "kernel_gradient": kernel_gradient,
    }


def numerical_kernel_gradient(
    signal: list[float],
    kernel: list[float],
    targets: list[float],
    epsilon: float,
) -> list[float]:
    result: list[float] = []
    for index in range(len(kernel)):
        plus = list(kernel)
        minus = list(kernel)
        plus[index] += epsilon
        minus[index] -= epsilon
        plus_loss = mean_squared_error(valid_correlation(signal, plus), targets)
        minus_loss = mean_squared_error(valid_correlation(signal, minus), targets)
        result.append((plus_loss - minus_loss) / (2.0 * epsilon))
    return result


def optimizer_step(
    signal: list[float],
    kernel: list[float],
    targets: list[float],
    learning_rate: float,
) -> dict[str, Any]:
    trace = trace_training(signal, kernel, targets)
    next_kernel = [
        value - learning_rate * gradient
        for value, gradient in zip(kernel, trace["kernel_gradient"])
    ]
    outputs = valid_correlation(signal, next_kernel)
    return {
        "kernel": next_kernel,
        "outputs": outputs,
        "loss": mean_squared_error(outputs, targets),
    }


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
            "targets",
            "loss",
            "gradient_check",
            "optimizer_step",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise ConvolutionTrainingValidationError(
            f"{source}.schema_version: expected 1"
        )
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise ConvolutionTrainingValidationError(
                f"{source}.{field}: expected a non-empty string"
            )
    concepts = lab["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(concepts) != len(set(concepts))
    ):
        raise ConvolutionTrainingValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )

    operation = lab["operation"]
    if not isinstance(operation, dict):
        raise ConvolutionTrainingValidationError(
            f"{source}.operation: expected an object"
        )
    _require_keys(operation, {"kind", "padding", "stride"}, f"{source}.operation")
    if operation != {
        "kind": "cross-correlation-1d",
        "padding": "valid",
        "stride": 1,
    }:
        raise ConvolutionTrainingValidationError(
            f"{source}.operation: V1 requires valid stride-one 1D cross-correlation"
        )

    signal = _numbers(lab["signal"], f"{source}.signal")
    kernel = _numbers(lab["kernel"], f"{source}.kernel")
    targets = _numbers(lab["targets"], f"{source}.targets")
    if len(kernel) > len(signal):
        raise ConvolutionTrainingValidationError(
            f"{source}.kernel: must be no longer than signal"
        )
    output_count = len(signal) - len(kernel) + 1
    if len(targets) != output_count:
        raise ConvolutionTrainingValidationError(
            f"{source}.targets: expected {output_count} values"
        )

    loss = lab["loss"]
    if not isinstance(loss, dict):
        raise ConvolutionTrainingValidationError(f"{source}.loss: expected an object")
    _require_keys(loss, {"kind", "reduction"}, f"{source}.loss")
    if loss != {"kind": "mean-squared-error", "reduction": "mean"}:
        raise ConvolutionTrainingValidationError(
            f"{source}.loss: V1 requires mean mean-squared-error"
        )

    check = lab["gradient_check"]
    if not isinstance(check, dict):
        raise ConvolutionTrainingValidationError(
            f"{source}.gradient_check: expected an object"
        )
    _require_keys(
        check, {"epsilon", "absolute_tolerance", "expected"},
        f"{source}.gradient_check",
    )
    _positive_number(check["epsilon"], f"{source}.gradient_check.epsilon")
    _positive_number(
        check["absolute_tolerance"],
        f"{source}.gradient_check.absolute_tolerance",
    )
    expected = check["expected"]
    if not isinstance(expected, dict):
        raise ConvolutionTrainingValidationError(
            f"{source}.gradient_check.expected: expected an object"
        )
    expected_keys = {
        "outputs",
        "errors",
        "loss",
        "output_gradients",
        "contributions",
        "kernel_gradient",
        "numerical_kernel_gradient",
    }
    _require_keys(expected, expected_keys, f"{source}.gradient_check.expected")
    for field, length in (
        ("outputs", output_count),
        ("errors", output_count),
        ("output_gradients", output_count),
        ("kernel_gradient", len(kernel)),
        ("numerical_kernel_gradient", len(kernel)),
    ):
        values = _numbers(expected[field], f"{source}.gradient_check.expected.{field}")
        if len(values) != length:
            raise ConvolutionTrainingValidationError(
                f"{source}.gradient_check.expected.{field}: expected {length} values"
            )
    _number(expected["loss"], f"{source}.gradient_check.expected.loss")
    contributions = expected["contributions"]
    if not isinstance(contributions, list) or len(contributions) != output_count:
        raise ConvolutionTrainingValidationError(
            f"{source}.gradient_check.expected.contributions: "
            f"expected {output_count} entries"
        )
    for index, contribution in enumerate(contributions):
        context = f"{source}.gradient_check.expected.contributions[{index}]"
        if not isinstance(contribution, dict):
            raise ConvolutionTrainingValidationError(f"{context}: expected an object")
        _require_keys(
            contribution,
            {"output_index", "window", "output_gradient", "kernel_gradient"},
            context,
        )
        if contribution["output_index"] != index:
            raise ConvolutionTrainingValidationError(
                f"{context}.output_index: expected {index}"
            )
        for field in ("window", "kernel_gradient"):
            if len(_numbers(contribution[field], f"{context}.{field}")) != len(kernel):
                raise ConvolutionTrainingValidationError(
                    f"{context}.{field}: expected {len(kernel)} values"
                )
        _number(contribution["output_gradient"], f"{context}.output_gradient")

    step = lab["optimizer_step"]
    if not isinstance(step, dict):
        raise ConvolutionTrainingValidationError(
            f"{source}.optimizer_step: expected an object"
        )
    _require_keys(
        step,
        {"learning_rate", "expected_kernel", "expected_outputs", "expected_loss"},
        f"{source}.optimizer_step",
    )
    _positive_number(step["learning_rate"], f"{source}.optimizer_step.learning_rate")
    if len(_numbers(step["expected_kernel"], f"{source}.optimizer_step.expected_kernel")) != len(kernel):
        raise ConvolutionTrainingValidationError(
            f"{source}.optimizer_step.expected_kernel: expected {len(kernel)} values"
        )
    if len(_numbers(step["expected_outputs"], f"{source}.optimizer_step.expected_outputs")) != output_count:
        raise ConvolutionTrainingValidationError(
            f"{source}.optimizer_step.expected_outputs: expected {output_count} values"
        )
    _number(step["expected_loss"], f"{source}.optimizer_step.expected_loss")


def _compare(actual: float, expected: Any, tolerance: float, context: str) -> None:
    expected_number = _number(expected, context)
    if abs(actual - expected_number) > tolerance:
        raise ConvolutionTrainingValidationError(
            f"{context}: expected {expected_number!r}, got {actual!r} "
            f"(tolerance {tolerance})"
        )


def _compare_arrays(
    actual: list[float], expected: Any, tolerance: float, context: str
) -> None:
    expected_values = _numbers(expected, context)
    for index, actual_value in enumerate(actual):
        _compare(actual_value, expected_values[index], tolerance, f"{context}[{index}]")


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    signal = _numbers(lab["signal"], f"{source}.signal")
    kernel = _numbers(lab["kernel"], f"{source}.kernel")
    targets = _numbers(lab["targets"], f"{source}.targets")
    check = lab["gradient_check"]
    expected = check["expected"]
    tolerance = float(check["absolute_tolerance"])
    actual = trace_training(signal, kernel, targets)
    for field in ("outputs", "errors", "output_gradients", "kernel_gradient"):
        _compare_arrays(
            actual[field], expected[field], tolerance,
            f"{source}.gradient_check.expected.{field}",
        )
    _compare(
        actual["loss"], expected["loss"], tolerance,
        f"{source}.gradient_check.expected.loss",
    )
    for index, contribution in enumerate(actual["contributions"]):
        expected_contribution = expected["contributions"][index]
        _compare_arrays(
            contribution["window"], expected_contribution["window"], tolerance,
            f"{source}.gradient_check.expected.contributions[{index}].window",
        )
        _compare(
            contribution["output_gradient"],
            expected_contribution["output_gradient"],
            tolerance,
            f"{source}.gradient_check.expected.contributions[{index}].output_gradient",
        )
        _compare_arrays(
            contribution["kernel_gradient"],
            expected_contribution["kernel_gradient"],
            tolerance,
            f"{source}.gradient_check.expected.contributions[{index}].kernel_gradient",
        )
    numerical = numerical_kernel_gradient(
        signal, kernel, targets, float(check["epsilon"])
    )
    _compare_arrays(
        numerical,
        expected["numerical_kernel_gradient"],
        tolerance,
        f"{source}.gradient_check.expected.numerical_kernel_gradient",
    )

    step = lab["optimizer_step"]
    actual_step = optimizer_step(
        signal, kernel, targets, float(step["learning_rate"])
    )
    _compare_arrays(
        actual_step["kernel"], step["expected_kernel"], tolerance,
        f"{source}.optimizer_step.expected_kernel",
    )
    _compare_arrays(
        actual_step["outputs"], step["expected_outputs"], tolerance,
        f"{source}.optimizer_step.expected_outputs",
    )
    _compare(
        actual_step["loss"], step["expected_loss"], tolerance,
        f"{source}.optimizer_step.expected_loss",
    )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise ConvolutionTrainingValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise ConvolutionTrainingValidationError(
            f"{fixture_root}: no lab fixtures found"
        )
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise ConvolutionTrainingValidationError(
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
    except ConvolutionTrainingValidationError as error:
        parser.exit(1, f"convolution training corpus invalid: {error}\n")
    print(f"validated {len(paths)} convolution training labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
