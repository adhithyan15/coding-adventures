#!/usr/bin/env python3
"""Validate and execute deterministic NN28 gradient-buffer schedules."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "gradient-accumulation-v1"
)
CASE_IDS = [
    "accumulate_two_calls",
    "zero_between_calls",
    "mean_then_zero",
    "stale_next_batch",
]
IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]{0,31}$")
MAX_SAMPLES = 4
MAX_EVENTS = 12
MAX_ABSOLUTE_INPUT = 1e3
MAX_ABSOLUTE_DERIVED = 1e12
CANONICAL_TOLERANCE = 1e-8
CANONICAL_EPSILON = 1e-5


class GradientAccumulationValidationError(ValueError):
    """Raised when an NN28 document or computed trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GradientAccumulationValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                GradientAccumulationValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise GradientAccumulationValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise GradientAccumulationValidationError("top-level JSON must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GradientAccumulationValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise GradientAccumulationValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise GradientAccumulationValidationError(
            f"{context}: expected non-empty string"
        )
    return value


def _identifier(value: Any, context: str) -> str:
    if not isinstance(value, str) or IDENTIFIER.fullmatch(value) is None:
        raise GradientAccumulationValidationError(f"{context}: invalid identifier")
    return value


def _number(value: Any, context: str, *, bounded: bool = True) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise GradientAccumulationValidationError(f"{context}: expected finite number")
    try:
        number = float(value)
    except (OverflowError, ValueError) as error:
        raise GradientAccumulationValidationError(
            f"{context}: expected finite number"
        ) from error
    if not math.isfinite(number):
        raise GradientAccumulationValidationError(f"{context}: expected finite number")
    if bounded and abs(number) > MAX_ABSOLUTE_INPUT:
        raise GradientAccumulationValidationError(
            f"{context}: magnitude exceeds {MAX_ABSOLUTE_INPUT:g}"
        )
    return number


def _finite(value: float, context: str) -> float:
    if not math.isfinite(value) or abs(value) > MAX_ABSOLUTE_DERIVED:
        raise GradientAccumulationValidationError(
            f"{context}: derived value is non-finite or unbounded"
        )
    return value


def _sample(value: Any, context: str) -> dict[str, Any]:
    item = _object(value, {"id", "input", "target"}, context)
    return {
        "id": _identifier(item["id"], f"{context}.id"),
        "input": _number(item["input"], f"{context}.input"),
        "target": _number(item["target"], f"{context}.target"),
    }


def _event(value: Any, sample_ids: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GradientAccumulationValidationError(f"{context}: expected object")
    kind = value.get("kind")
    if kind == "backward":
        event = _object(value, {"kind", "sample_id"}, context)
        sample_id = _identifier(event["sample_id"], f"{context}.sample_id")
        if sample_id not in sample_ids:
            raise GradientAccumulationValidationError(
                f"{context}.sample_id: unknown sample {sample_id}"
            )
        return {"kind": "backward", "sample_id": sample_id}
    if kind == "zero_grad":
        _object(value, {"kind"}, context)
        return {"kind": "zero_grad"}
    if kind == "optimizer_step":
        event = _object(value, {"kind", "divisor"}, context)
        divisor = event["divisor"]
        if (
            isinstance(divisor, bool)
            or not isinstance(divisor, int)
            or not 1 <= divisor <= MAX_SAMPLES
        ):
            raise GradientAccumulationValidationError(
                f"{context}.divisor: expected integer in [1, {MAX_SAMPLES}]"
            )
        return {"kind": "optimizer_step", "divisor": divisor}
    raise GradientAccumulationValidationError(f"{context}.kind: unsupported event")


def _validate_case(value: Any, context: str) -> dict[str, Any]:
    case = _object(
        value,
        {
            "id",
            "title",
            "initial_parameter",
            "learning_rate",
            "samples",
            "events",
            "expected",
        },
        context,
    )
    initial_parameter = _number(
        case["initial_parameter"], f"{context}.initial_parameter"
    )
    learning_rate = _number(case["learning_rate"], f"{context}.learning_rate")
    if not 0 < learning_rate <= 1:
        raise GradientAccumulationValidationError(
            f"{context}.learning_rate: expected value in (0, 1]"
        )
    raw_samples = case["samples"]
    if not isinstance(raw_samples, list) or not 1 <= len(raw_samples) <= MAX_SAMPLES:
        raise GradientAccumulationValidationError(
            f"{context}.samples: expected 1 to {MAX_SAMPLES} samples"
        )
    samples = [
        _sample(sample, f"{context}.samples[{index}]")
        for index, sample in enumerate(raw_samples)
    ]
    sample_ids = [sample["id"] for sample in samples]
    if len(set(sample_ids)) != len(sample_ids):
        raise GradientAccumulationValidationError(
            f"{context}.samples: duplicate identifiers"
        )
    raw_events = case["events"]
    if not isinstance(raw_events, list) or not 1 <= len(raw_events) <= MAX_EVENTS:
        raise GradientAccumulationValidationError(
            f"{context}.events: expected 1 to {MAX_EVENTS} events"
        )
    events = [
        _event(event, set(sample_ids), f"{context}.events[{index}]")
        for index, event in enumerate(raw_events)
    ]
    if not any(event["kind"] == "backward" for event in events):
        raise GradientAccumulationValidationError(
            f"{context}.events: expected at least one backward call"
        )
    if not isinstance(case["expected"], dict):
        raise GradientAccumulationValidationError(
            f"{context}.expected: expected object"
        )
    return {
        "id": _identifier(case["id"], f"{context}.id"),
        "title": _text(case["title"], f"{context}.title"),
        "initial_parameter": initial_parameter,
        "learning_rate": learning_rate,
        "samples": samples,
        "events": events,
        "expected": case["expected"],
    }


def _sample_loss(parameter: float, sample: dict[str, Any]) -> float:
    prediction = _finite(parameter * sample["input"], "finite-difference prediction")
    residual = _finite(prediction - sample["target"], "finite-difference residual")
    return _finite(0.5 * residual * residual, "finite-difference loss")


def execute_case(
    case: dict[str, Any], finite_difference_epsilon: float
) -> dict[str, Any]:
    epsilon = _number(
        finite_difference_epsilon, "finite-difference epsilon", bounded=False
    )
    if not 1e-12 <= epsilon <= 1:
        raise GradientAccumulationValidationError(
            "finite-difference epsilon must be in [1e-12, 1]"
        )
    samples = {sample["id"]: sample for sample in case["samples"]}
    parameter = case["initial_parameter"]
    gradient_buffer = 0.0
    steps: list[dict[str, Any]] = []
    errors: list[float] = []

    for index, event in enumerate(case["events"]):
        parameter_before = parameter
        buffer_before = gradient_buffer
        if event["kind"] == "backward":
            sample = samples[event["sample_id"]]
            prediction = _finite(
                parameter * sample["input"], f"event {index} prediction"
            )
            residual = _finite(prediction - sample["target"], f"event {index} residual")
            loss = _finite(0.5 * residual * residual, f"event {index} loss")
            local_gradient = _finite(
                residual * sample["input"], f"event {index} local gradient"
            )
            gradient_buffer = _finite(
                gradient_buffer + local_gradient, f"event {index} gradient buffer"
            )
            numerical_gradient = _finite(
                (
                    _sample_loss(parameter + epsilon, sample)
                    - _sample_loss(parameter - epsilon, sample)
                )
                / (2 * epsilon),
                f"event {index} numerical gradient",
            )
            error = abs(local_gradient - numerical_gradient)
            errors.append(error)
            steps.append(
                {
                    "index": index,
                    "kind": "backward",
                    "sample_id": sample["id"],
                    "parameter_before": parameter_before,
                    "parameter_after": parameter,
                    "buffer_before": buffer_before,
                    "buffer_after": gradient_buffer,
                    "prediction": prediction,
                    "residual": residual,
                    "loss": loss,
                    "local_gradient": local_gradient,
                    "numerical_gradient": numerical_gradient,
                    "gradient_absolute_error": error,
                }
            )
        elif event["kind"] == "zero_grad":
            gradient_buffer = 0.0
            steps.append(
                {
                    "index": index,
                    "kind": "zero_grad",
                    "parameter_before": parameter_before,
                    "parameter_after": parameter,
                    "buffer_before": buffer_before,
                    "buffer_after": gradient_buffer,
                }
            )
        else:
            applied_gradient = _finite(
                gradient_buffer / event["divisor"],
                f"event {index} applied gradient",
            )
            parameter_delta = _finite(
                -case["learning_rate"] * applied_gradient,
                f"event {index} parameter delta",
            )
            parameter = _finite(parameter + parameter_delta, f"event {index} parameter")
            steps.append(
                {
                    "index": index,
                    "kind": "optimizer_step",
                    "parameter_before": parameter_before,
                    "parameter_after": parameter,
                    "buffer_before": buffer_before,
                    "buffer_after": gradient_buffer,
                    "divisor": event["divisor"],
                    "applied_gradient": applied_gradient,
                    "parameter_delta": parameter_delta,
                }
            )
    return {
        "steps": steps,
        "final_parameter": parameter,
        "final_gradient_buffer": gradient_buffer,
        "max_gradient_absolute_error": max(errors, default=0.0),
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise GradientAccumulationValidationError(f"{context}: value mismatch")
    elif isinstance(expected, (int, float)):
        expected_number = _number(expected, f"{context} expected", bounded=False)
        actual_number = _number(actual, context, bounded=False)
        if abs(actual_number - expected_number) > tolerance:
            raise GradientAccumulationValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise GradientAccumulationValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise GradientAccumulationValidationError(f"{context}: list mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            actual_keys = sorted(actual) if isinstance(actual, dict) else []
            raise GradientAccumulationValidationError(
                f"{context}: object keys expected {sorted(expected)}, got {actual_keys}"
            )
        for key, value in expected.items():
            _compare(actual[key], value, tolerance, f"{context}.{key}")
    else:
        raise GradientAccumulationValidationError(
            f"{context}: unsupported expected value"
        )


def validate_document(document: dict[str, Any]) -> dict[str, Any]:
    lab = _object(
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
        "lab",
    )
    if lab["schema_version"] != 1:
        raise GradientAccumulationValidationError("lab.schema_version: expected 1")
    _text(lab["id"], "lab.id")
    _text(lab["title"], "lab.title")
    _text(lab["question"], "lab.question")
    tolerance = _number(lab["absolute_tolerance"], "lab.absolute_tolerance")
    if tolerance != CANONICAL_TOLERANCE:
        raise GradientAccumulationValidationError(
            f"lab.absolute_tolerance: expected canonical {CANONICAL_TOLERANCE}"
        )
    operation = _object(
        lab["operation"],
        {
            "kind",
            "loss",
            "backward",
            "optimizer",
            "optimizer_step_zeroes_gradient",
            "finite_difference_epsilon",
        },
        "lab.operation",
    )
    canonical_operation = {
        "kind": "persistent-scalar-gradient-buffer",
        "loss": "half-squared-error",
        "backward": "add-to-buffer",
        "optimizer": "sgd",
        "optimizer_step_zeroes_gradient": False,
        "finite_difference_epsilon": CANONICAL_EPSILON,
    }
    _compare(operation, canonical_operation, 0.0, "lab.operation")
    raw_cases = lab["cases"]
    if not isinstance(raw_cases, list) or len(raw_cases) != len(CASE_IDS):
        raise GradientAccumulationValidationError(
            f"lab.cases: expected {len(CASE_IDS)} cases"
        )
    cases = [
        _validate_case(case, f"lab.cases[{index}]")
        for index, case in enumerate(raw_cases)
    ]
    case_ids = [case["id"] for case in cases]
    if case_ids != CASE_IDS:
        raise GradientAccumulationValidationError(
            f"lab.cases: case ids expected {CASE_IDS}, got {case_ids}"
        )
    for case in cases:
        trace = execute_case(case, CANONICAL_EPSILON)
        if trace["max_gradient_absolute_error"] > tolerance:
            raise GradientAccumulationValidationError(
                f"case {case['id']}: numerical gradient error "
                f"{trace['max_gradient_absolute_error']!r} exceeds {tolerance!r}"
            )
        _compare(trace, case["expected"], tolerance, f"case {case['id']}.expected")
    return {**lab, "cases": cases}


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise GradientAccumulationValidationError(f"{root}: no lab JSON files")
    for path in paths:
        validate_document(load_json(path))
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    try:
        count = validate_fixture_root(args.root)
    except GradientAccumulationValidationError as error:
        parser.error(str(error))
    print(f"validated {count} gradient accumulation lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
