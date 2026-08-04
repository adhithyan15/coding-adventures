#!/usr/bin/env python3
"""Validate and execute deterministic NN24 gradient-flow labs."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "gradient-flow-v1"
SCENARIO_IDS = ["small-tanh", "saturated-tanh", "unit-relu", "large-relu"]


class GradientFlowValidationError(ValueError):
    """Raised when an NN24 document or trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GradientFlowValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                GradientFlowValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise GradientFlowValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise GradientFlowValidationError("top-level JSON must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GradientFlowValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise GradientFlowValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(float(value))
    ):
        raise GradientFlowValidationError(f"{context}: expected finite number")
    return float(value)


def _scenario(value: Any, index: int) -> dict[str, Any]:
    scenario = _object(
        value,
        {"id", "label", "input", "weights", "activation", "target"},
        f"scenarios[{index}]",
    )
    for key in ("id", "label"):
        if not isinstance(scenario[key], str) or not scenario[key]:
            raise GradientFlowValidationError(
                f"scenarios[{index}].{key}: expected non-empty string"
            )
    scenario_input = _number(scenario["input"], f"scenarios[{index}].input")
    target = _number(scenario["target"], f"scenarios[{index}].target")
    weights = scenario["weights"]
    if not isinstance(weights, list) or len(weights) < 2:
        raise GradientFlowValidationError(
            f"scenarios[{index}].weights: expected at least two values"
        )
    numeric_weights = [
        _number(weight, f"scenarios[{index}].weights[{weight_index}]")
        for weight_index, weight in enumerate(weights)
    ]
    if scenario["activation"] not in {"tanh", "relu"}:
        raise GradientFlowValidationError(
            f"scenarios[{index}].activation: expected tanh or relu"
        )
    return {
        "id": scenario["id"],
        "label": scenario["label"],
        "input": scenario_input,
        "weights": numeric_weights,
        "activation": scenario["activation"],
        "target": target,
    }


def _activate(value: float, activation: str) -> float:
    return math.tanh(value) if activation == "tanh" else max(0.0, value)


def _derivative(preactivation: float, activation: float, kind: str) -> float:
    return 1 - activation**2 if kind == "tanh" else (1.0 if preactivation > 0 else 0.0)


def _loss_at_input(scenario: dict[str, Any], scenario_input: float) -> float:
    current = scenario_input
    for weight in scenario["weights"]:
        current = _activate(weight * current, scenario["activation"])
    return 0.5 * (current - scenario["target"]) ** 2


def _trace(scenario: dict[str, Any], epsilon: float) -> dict[str, Any]:
    current = scenario["input"]
    layers = []
    for layer_index, weight in enumerate(scenario["weights"], start=1):
        preactivation = weight * current
        activation = _activate(preactivation, scenario["activation"])
        activation_derivative = _derivative(
            preactivation, activation, scenario["activation"]
        )
        layers.append(
            {
                "layer": layer_index,
                "input": current,
                "weight": weight,
                "preactivation": preactivation,
                "activation": activation,
                "activation_derivative": activation_derivative,
                "local_jacobian": weight * activation_derivative,
                "upstream_gradient": 0.0,
                "preactivation_gradient": 0.0,
                "weight_gradient": 0.0,
                "input_gradient": 0.0,
            }
        )
        current = activation
    output = current
    output_error = output - scenario["target"]
    loss = 0.5 * output_error**2
    upstream = output_error
    for layer in reversed(layers):
        preactivation_gradient = upstream * layer["activation_derivative"]
        layer["upstream_gradient"] = upstream
        layer["preactivation_gradient"] = preactivation_gradient
        layer["weight_gradient"] = preactivation_gradient * layer["input"]
        layer["input_gradient"] = preactivation_gradient * layer["weight"]
        upstream = layer["input_gradient"]
    chain_jacobian = math.prod(layer["local_jacobian"] for layer in layers)
    finite_difference = (
        _loss_at_input(scenario, scenario["input"] + epsilon)
        - _loss_at_input(scenario, scenario["input"] - epsilon)
    ) / (2 * epsilon)
    magnitude = abs(chain_jacobian)
    classification = (
        "vanishing" if magnitude < 0.1 else "exploding" if magnitude > 10 else "stable"
    )
    return {
        "id": scenario["id"],
        "output": output,
        "output_error": output_error,
        "loss": loss,
        "chain_jacobian": chain_jacobian,
        "input_gradient": upstream,
        "finite_difference_input_gradient": finite_difference,
        "finite_difference_error": abs(upstream - finite_difference),
        "classification": classification,
        "layers": layers,
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    epsilon = _number(
        document["operation"]["finite_difference_epsilon"],
        "operation.finite_difference_epsilon",
    )
    if epsilon <= 0:
        raise GradientFlowValidationError(
            "operation.finite_difference_epsilon must be positive"
        )
    raw_scenarios = document["scenarios"]
    if not isinstance(raw_scenarios, list) or len(raw_scenarios) != 4:
        raise GradientFlowValidationError("scenarios: expected four scenarios")
    scenarios = [
        _scenario(raw_scenario, index)
        for index, raw_scenario in enumerate(raw_scenarios)
    ]
    if [scenario["id"] for scenario in scenarios] != SCENARIO_IDS:
        raise GradientFlowValidationError("scenario order does not match NN24 V1")
    return {"traces": [_trace(scenario, epsilon) for scenario in scenarios]}


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise GradientFlowValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        if abs(_number(actual, context) - float(expected)) > tolerance:
            raise GradientFlowValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise GradientFlowValidationError(f"{context}: mismatch")
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise GradientFlowValidationError(f"{context}: array length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        value = _object(actual, set(expected), context)
        for key, right in expected.items():
            _compare(value[key], right, tolerance, f"{context}.{key}")
    else:
        raise GradientFlowValidationError(f"{context}: unsupported value")


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
            "scenarios",
            "expected",
        },
        "document",
    )
    if root["schema_version"] != 1:
        raise GradientFlowValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise GradientFlowValidationError(f"{key}: expected non-empty string")
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise GradientFlowValidationError("absolute_tolerance must be positive")
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or len(concepts) != len(set(concepts))
        or any(not isinstance(item, str) or not item for item in concepts)
    ):
        raise GradientFlowValidationError("concepts must be unique non-empty strings")
    operation = _object(
        root["operation"],
        {
            "kind",
            "loss",
            "activation_derivative",
            "finite_difference_epsilon",
            "vanishing_threshold",
            "exploding_threshold",
        },
        "operation",
    )
    required = {
        "kind": "scalar-gradient-flow",
        "loss": "half-squared-error",
        "activation_derivative": "saved-output",
        "finite_difference_epsilon": 1e-6,
        "vanishing_threshold": 0.1,
        "exploding_threshold": 10,
    }
    if operation != required:
        raise GradientFlowValidationError("operation does not match NN24 V1")
    actual = execute_lab(root)
    if not isinstance(root["expected"], dict):
        raise GradientFlowValidationError("expected must be object")
    _compare(actual, root["expected"], tolerance, "expected")
    if any(trace["finite_difference_error"] > 1e-8 for trace in actual["traces"]):
        raise GradientFlowValidationError("input gradient finite difference failed")
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    load_json(root / "schema.json")
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise GradientFlowValidationError("no labs found")
    for path in paths:
        validate_document(load_json(path))
        print(f"validated {path}")
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN24 gradient-flow lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
