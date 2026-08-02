#!/usr/bin/env python3
"""Validate and execute the deterministic NN09 recurrent-unroll corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "recurrent-unroll-v1"
)


class RecurrentUnrollValidationError(ValueError):
    """Raised when an NN09 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise RecurrentUnrollValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                RecurrentUnrollValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise RecurrentUnrollValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise RecurrentUnrollValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise RecurrentUnrollValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise RecurrentUnrollValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise RecurrentUnrollValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise RecurrentUnrollValidationError(f"{context}: expected a finite number")
    return result


def _numbers(value: Any, context: str, length: int | None = None) -> list[float]:
    if not isinstance(value, list) or not value:
        raise RecurrentUnrollValidationError(
            f"{context}: expected a non-empty number array"
        )
    result = [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]
    if length is not None and len(result) != length:
        raise RecurrentUnrollValidationError(
            f"{context}: expected {length} values"
        )
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise RecurrentUnrollValidationError(
            f"{context}: expected a positive number"
        )
    return result


def trace_recurrent(
    inputs: list[float],
    initial_state: float,
    parameters: dict[str, float],
    *,
    recurrent_enabled: bool = True,
) -> dict[str, Any]:
    """Run one shared scalar ReLU cell over an input sequence."""

    if not inputs:
        raise RecurrentUnrollValidationError("recurrent trace needs an input")
    _require_keys(
        parameters,
        {"input_weight", "recurrent_weight", "bias"},
        "parameters",
    )
    input_weight = _number(parameters["input_weight"], "parameters.input_weight")
    recurrent_weight = _number(
        parameters["recurrent_weight"], "parameters.recurrent_weight"
    )
    bias = _number(parameters["bias"], "parameters.bias")
    previous_state = _number(initial_state, "initial_state")
    steps: list[dict[str, float | int]] = []
    states: list[float] = []
    for time, input_value in enumerate(inputs):
        input_number = _number(input_value, f"inputs[{time}]")
        input_product = input_weight * input_number
        recurrent_product = (
            recurrent_weight * previous_state if recurrent_enabled else 0.0
        )
        preactivation = input_product + recurrent_product + bias
        state = max(0.0, preactivation)
        steps.append(
            {
                "time": time,
                "input": input_number,
                "previous_state": previous_state,
                "input_product": input_product,
                "recurrent_product": recurrent_product,
                "bias": bias,
                "preactivation": preactivation,
                "state": state,
            }
        )
        states.append(state)
        previous_state = state
    return {"steps": steps, "states": states, "final_state": states[-1]}


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
            "initial_state",
            "inputs",
            "parameters",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise RecurrentUnrollValidationError(f"{source}.schema_version: expected 1")
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise RecurrentUnrollValidationError(
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
        raise RecurrentUnrollValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )

    expected_operation = {
        "kind": "scalar-elman-rnn-forward",
        "steps": 3,
        "state_size": 1,
        "activation": "relu",
        "parameter_sharing": "all-steps",
    }
    operation = lab["operation"]
    if not isinstance(operation, dict):
        raise RecurrentUnrollValidationError(f"{source}.operation: expected an object")
    _require_keys(operation, set(expected_operation), f"{source}.operation")
    if operation != expected_operation:
        raise RecurrentUnrollValidationError(
            f"{source}.operation: unsupported V1 operation"
        )

    _number(lab["initial_state"], f"{source}.initial_state")
    _numbers(lab["inputs"], f"{source}.inputs", 3)
    parameters = lab["parameters"]
    if not isinstance(parameters, dict):
        raise RecurrentUnrollValidationError(
            f"{source}.parameters: expected an object"
        )
    _require_keys(
        parameters,
        {"input_weight", "recurrent_weight", "bias"},
        f"{source}.parameters",
    )
    for field in ("input_weight", "recurrent_weight", "bias"):
        _number(parameters[field], f"{source}.parameters.{field}")

    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise RecurrentUnrollValidationError(f"{source}.expected: expected an object")
    _require_keys(
        expected,
        {"steps", "states", "final_state", "memory_ablation"},
        f"{source}.expected",
    )
    steps = expected["steps"]
    if not isinstance(steps, list) or len(steps) != 3:
        raise RecurrentUnrollValidationError(
            f"{source}.expected.steps: expected three steps"
        )
    step_keys = {
        "time",
        "input",
        "previous_state",
        "input_product",
        "recurrent_product",
        "bias",
        "preactivation",
        "state",
    }
    for time, step in enumerate(steps):
        context = f"{source}.expected.steps[{time}]"
        if not isinstance(step, dict):
            raise RecurrentUnrollValidationError(f"{context}: expected an object")
        _require_keys(step, step_keys, context)
        if step["time"] != time:
            raise RecurrentUnrollValidationError(f"{context}.time: expected {time}")
        for field in step_keys - {"time"}:
            _number(step[field], f"{context}.{field}")
    _numbers(expected["states"], f"{source}.expected.states", 3)
    _number(expected["final_state"], f"{source}.expected.final_state")
    ablation = expected["memory_ablation"]
    if not isinstance(ablation, dict):
        raise RecurrentUnrollValidationError(
            f"{source}.expected.memory_ablation: expected an object"
        )
    _require_keys(
        ablation,
        {"preactivations", "states", "state_differences"},
        f"{source}.expected.memory_ablation",
    )
    for field in ("preactivations", "states", "state_differences"):
        _numbers(ablation[field], f"{source}.expected.memory_ablation.{field}", 3)


def _compare_nested(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(actual, dict):
        if not isinstance(expected, dict) or actual.keys() != expected.keys():
            raise RecurrentUnrollValidationError(f"{context}: object keys do not match")
        for key, actual_value in actual.items():
            _compare_nested(actual_value, expected[key], tolerance, f"{context}.{key}")
        return
    if isinstance(actual, list):
        if not isinstance(expected, list) or len(actual) != len(expected):
            raise RecurrentUnrollValidationError(f"{context}: list lengths do not match")
        for index, actual_value in enumerate(actual):
            _compare_nested(
                actual_value, expected[index], tolerance, f"{context}[{index}]"
            )
        return
    actual_number = _number(actual, context)
    expected_number = _number(expected, context)
    if abs(actual_number - expected_number) > tolerance:
        raise RecurrentUnrollValidationError(
            f"{context}: expected {expected_number!r}, got {actual_number!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    inputs = [float(value) for value in lab["inputs"]]
    initial_state = float(lab["initial_state"])
    parameters = {key: float(value) for key, value in lab["parameters"].items()}
    forward = trace_recurrent(inputs, initial_state, parameters)
    ablated = trace_recurrent(
        inputs, initial_state, parameters, recurrent_enabled=False
    )
    actual = {
        **forward,
        "memory_ablation": {
            "preactivations": [step["preactivation"] for step in ablated["steps"]],
            "states": ablated["states"],
            "state_differences": [
                state - ablated_state
                for state, ablated_state in zip(
                    forward["states"], ablated["states"]
                )
            ],
        },
    }
    _compare_nested(
        actual,
        lab["expected"],
        float(lab["absolute_tolerance"]),
        f"{source}.expected",
    )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise RecurrentUnrollValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise RecurrentUnrollValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise RecurrentUnrollValidationError(
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
    except RecurrentUnrollValidationError as error:
        parser.exit(1, f"recurrent-unroll corpus invalid: {error}\n")
    print(f"validated {len(paths)} recurrent-unroll labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
