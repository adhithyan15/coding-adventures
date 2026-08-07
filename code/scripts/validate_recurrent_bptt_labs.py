#!/usr/bin/env python3
"""Validate and execute the deterministic NN10 recurrent BPTT corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "recurrent-bptt-v1"
)
PARAMETER_KEYS = ("input_weight", "recurrent_weight", "bias")


class RecurrentBpttValidationError(ValueError):
    """Raised when an NN10 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise RecurrentBpttValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                RecurrentBpttValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise RecurrentBpttValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise RecurrentBpttValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise RecurrentBpttValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise RecurrentBpttValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise RecurrentBpttValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise RecurrentBpttValidationError(f"{context}: expected a finite number")
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise RecurrentBpttValidationError(
            f"{context}: expected a positive number"
        )
    return result


def _numbers(value: Any, context: str, length: int) -> list[float]:
    if not isinstance(value, list) or len(value) != length:
        raise RecurrentBpttValidationError(
            f"{context}: expected {length} numbers"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _parameters(value: Any, context: str) -> dict[str, float]:
    if not isinstance(value, dict):
        raise RecurrentBpttValidationError(f"{context}: expected an object")
    _require_keys(value, set(PARAMETER_KEYS), context)
    return {key: _number(value[key], f"{context}.{key}") for key in PARAMETER_KEYS}


def forward_trace(
    inputs: list[float],
    initial_state: float,
    parameters: dict[str, float],
    target: float,
) -> dict[str, Any]:
    previous_state = initial_state
    preactivations: list[float] = []
    states: list[float] = []
    for input_value in inputs:
        preactivation = (
            parameters["input_weight"] * input_value
            + parameters["recurrent_weight"] * previous_state
            + parameters["bias"]
        )
        state = max(0.0, preactivation)
        preactivations.append(preactivation)
        states.append(state)
        previous_state = state
    prediction = states[-1]
    return {
        "preactivations": preactivations,
        "states": states,
        "prediction": prediction,
        "loss": 0.5 * (prediction - target) ** 2,
    }


def bptt_trace(
    inputs: list[float],
    initial_state: float,
    parameters: dict[str, float],
    target: float,
) -> dict[str, Any]:
    forward = forward_trace(inputs, initial_state, parameters, target)
    backward_steps: list[dict[str, float | int]] = []
    future_state_gradient = 0.0
    input_weight_gradient = 0.0
    recurrent_weight_gradient = 0.0
    bias_gradient = 0.0
    final_time = len(inputs) - 1
    for time in range(final_time, -1, -1):
        direct_loss_gradient = (
            forward["prediction"] - target if time == final_time else 0.0
        )
        state_gradient = direct_loss_gradient + future_state_gradient
        relu_derivative = 1.0 if forward["preactivations"][time] > 0 else 0.0
        preactivation_gradient = state_gradient * relu_derivative
        previous_state = initial_state if time == 0 else forward["states"][time - 1]
        local_input_weight_gradient = preactivation_gradient * inputs[time]
        local_recurrent_weight_gradient = preactivation_gradient * previous_state
        local_bias_gradient = preactivation_gradient
        previous_state_gradient = (
            preactivation_gradient * parameters["recurrent_weight"]
        )
        backward_steps.append(
            {
                "time": time,
                "direct_loss_gradient": direct_loss_gradient,
                "future_state_gradient": future_state_gradient,
                "state_gradient": state_gradient,
                "relu_derivative": relu_derivative,
                "preactivation_gradient": preactivation_gradient,
                "input_weight_gradient": local_input_weight_gradient,
                "recurrent_weight_gradient": local_recurrent_weight_gradient,
                "bias_gradient": local_bias_gradient,
                "previous_state_gradient": previous_state_gradient,
            }
        )
        input_weight_gradient += local_input_weight_gradient
        recurrent_weight_gradient += local_recurrent_weight_gradient
        bias_gradient += local_bias_gradient
        future_state_gradient = previous_state_gradient
    return {
        "forward": forward,
        "backward_steps": backward_steps,
        "gradient_totals": {
            "input_weight": input_weight_gradient,
            "recurrent_weight": recurrent_weight_gradient,
            "bias": bias_gradient,
            "initial_state": future_state_gradient,
        },
    }


def numerical_gradients(
    inputs: list[float],
    initial_state: float,
    parameters: dict[str, float],
    target: float,
    epsilon: float,
) -> dict[str, float]:
    gradients: dict[str, float] = {}
    for key in PARAMETER_KEYS:
        plus = dict(parameters)
        minus = dict(parameters)
        plus[key] += epsilon
        minus[key] -= epsilon
        plus_loss = forward_trace(inputs, initial_state, plus, target)["loss"]
        minus_loss = forward_trace(inputs, initial_state, minus, target)["loss"]
        gradients[key] = (plus_loss - minus_loss) / (2 * epsilon)
    return gradients


def execute_lab(lab: dict[str, Any]) -> dict[str, Any]:
    inputs = [float(value) for value in lab["inputs"]]
    initial_state = float(lab["initial_state"])
    target = float(lab["target"])
    parameters = {key: float(value) for key, value in lab["parameters"].items()}
    trace = bptt_trace(inputs, initial_state, parameters, target)
    numerical = numerical_gradients(
        inputs,
        initial_state,
        parameters,
        target,
        float(lab["finite_difference_epsilon"]),
    )
    analytical = trace["gradient_totals"]
    errors = {
        key: abs(numerical[key] - analytical[key]) for key in PARAMETER_KEYS
    }
    next_parameters = {
        key: parameters[key] - float(lab["learning_rate"]) * analytical[key]
        for key in PARAMETER_KEYS
    }
    updated_forward = forward_trace(
        inputs, initial_state, next_parameters, target
    )
    return {
        **trace,
        "gradient_check": {
            "numerical": numerical,
            "absolute_errors": errors,
            "max_absolute_error": max(errors.values()),
        },
        "update": {
            "parameters": next_parameters,
            "preactivations": updated_forward["preactivations"],
            "states": updated_forward["states"],
            "loss": updated_forward["loss"],
        },
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
            "initial_state",
            "inputs",
            "target",
            "parameters",
            "learning_rate",
            "finite_difference_epsilon",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise RecurrentBpttValidationError(f"{source}.schema_version: expected 1")
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise RecurrentBpttValidationError(
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
        raise RecurrentBpttValidationError(
            f"{source}.concepts: expected unique non-empty strings"
        )
    expected_operation = {
        "kind": "scalar-elman-rnn-bptt",
        "steps": 3,
        "activation": "relu",
        "loss": "half-squared-error",
        "parameter_sharing": "all-steps",
    }
    operation = lab["operation"]
    if not isinstance(operation, dict):
        raise RecurrentBpttValidationError(f"{source}.operation: expected an object")
    _require_keys(operation, set(expected_operation), f"{source}.operation")
    if operation != expected_operation:
        raise RecurrentBpttValidationError(
            f"{source}.operation: unsupported V1 operation"
        )
    _number(lab["initial_state"], f"{source}.initial_state")
    _numbers(lab["inputs"], f"{source}.inputs", 3)
    _number(lab["target"], f"{source}.target")
    _parameters(lab["parameters"], f"{source}.parameters")
    _positive_number(lab["learning_rate"], f"{source}.learning_rate")
    _positive_number(
        lab["finite_difference_epsilon"],
        f"{source}.finite_difference_epsilon",
    )
    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise RecurrentBpttValidationError(f"{source}.expected: expected an object")
    _require_keys(
        expected,
        {"forward", "backward_steps", "gradient_totals", "gradient_check", "update"},
        f"{source}.expected",
    )
    steps = expected["backward_steps"]
    if not isinstance(steps, list) or len(steps) != 3:
        raise RecurrentBpttValidationError(
            f"{source}.expected.backward_steps: expected three steps"
        )
    if [step.get("time") if isinstance(step, dict) else None for step in steps] != [2, 1, 0]:
        raise RecurrentBpttValidationError(
            f"{source}.expected.backward_steps: expected reverse time order"
        )


def _compare_nested(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(actual, dict):
        if not isinstance(expected, dict) or actual.keys() != expected.keys():
            raise RecurrentBpttValidationError(f"{context}: object keys do not match")
        for key, actual_value in actual.items():
            _compare_nested(actual_value, expected[key], tolerance, f"{context}.{key}")
        return
    if isinstance(actual, list):
        if not isinstance(expected, list) or len(actual) != len(expected):
            raise RecurrentBpttValidationError(f"{context}: list lengths do not match")
        for index, actual_value in enumerate(actual):
            _compare_nested(
                actual_value, expected[index], tolerance, f"{context}[{index}]"
            )
        return
    actual_number = _number(actual, context)
    expected_number = _number(expected, context)
    if abs(actual_number - expected_number) > tolerance:
        raise RecurrentBpttValidationError(
            f"{context}: expected {expected_number!r}, got {actual_number!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    _compare_nested(
        execute_lab(lab),
        lab["expected"],
        float(lab["absolute_tolerance"]),
        f"{source}.expected",
    )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise RecurrentBpttValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise RecurrentBpttValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise RecurrentBpttValidationError(
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
    except RecurrentBpttValidationError as error:
        parser.exit(1, f"recurrent BPTT corpus invalid: {error}\n")
    print(f"validated {len(paths)} recurrent BPTT labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
