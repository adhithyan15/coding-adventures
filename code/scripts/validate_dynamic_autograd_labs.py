#!/usr/bin/env python3
"""Validate and execute deterministic NN27 dynamic-autograd labs."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "dynamic-autograd-v1"
CASE_IDS = ["multiply_add_square", "negative_branch", "saved_snapshot"]
OPERATIONS = {"multiply", "add", "square", "negate", "branch_nonnegative"}
ARITY = {"multiply": 2, "add": 2, "square": 1, "negate": 1, "branch_nonnegative": 1}
IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]{0,31}$")
MAX_ABSOLUTE_VALUE = 1e6
MAX_INPUTS = 4
MAX_STEPS = 12
CANONICAL_TOLERANCE = 1e-8
CANONICAL_EPSILON = 1e-5


class DynamicAutogradValidationError(ValueError):
    """Raised when an NN27 document or computed trace is invalid."""


def _duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DynamicAutogradValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicates,
            parse_constant=lambda item: (_ for _ in ()).throw(
                DynamicAutogradValidationError(f"non-finite JSON number: {item}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise DynamicAutogradValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise DynamicAutogradValidationError("top-level JSON must be an object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise DynamicAutogradValidationError(f"{context}: expected object")
    missing, extra = keys - value.keys(), value.keys() - keys
    if missing or extra:
        raise DynamicAutogradValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _identifier(value: Any, context: str) -> str:
    if not isinstance(value, str) or IDENTIFIER.fullmatch(value) is None:
        raise DynamicAutogradValidationError(f"{context}: invalid identifier")
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise DynamicAutogradValidationError(f"{context}: expected non-empty string")
    return value


def _number(value: Any, context: str, *, bounded: bool = True) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise DynamicAutogradValidationError(f"{context}: expected finite number")
    try:
        number = float(value)
    except (OverflowError, ValueError) as error:
        raise DynamicAutogradValidationError(
            f"{context}: expected finite number"
        ) from error
    if not math.isfinite(number):
        raise DynamicAutogradValidationError(f"{context}: expected finite number")
    if bounded and abs(number) > MAX_ABSOLUTE_VALUE:
        raise DynamicAutogradValidationError(
            f"{context}: magnitude exceeds {MAX_ABSOLUTE_VALUE:g}"
        )
    return number


def _finite(value: float, context: str) -> float:
    if not math.isfinite(value):
        raise DynamicAutogradValidationError(f"{context}: derived value is non-finite")
    return value


def _input(value: Any, context: str) -> dict[str, Any]:
    item = _object(value, {"id", "value", "requires_gradient"}, context)
    if item["requires_gradient"] is not True:
        raise DynamicAutogradValidationError(
            f"{context}.requires_gradient: expected true"
        )
    return {
        "id": _identifier(item["id"], f"{context}.id"),
        "value": _number(item["value"], f"{context}.value"),
        "requires_gradient": True,
    }


def _step(value: Any, known: set[str], context: str) -> dict[str, Any]:
    step = _object(value, {"id", "operation", "inputs"}, context)
    step_id = _identifier(step["id"], f"{context}.id")
    if step_id in known:
        raise DynamicAutogradValidationError(f"{context}.id: duplicate node {step_id}")
    operation = step["operation"]
    if operation not in OPERATIONS:
        raise DynamicAutogradValidationError(
            f"{context}.operation: unsupported operation"
        )
    raw_inputs = step["inputs"]
    if not isinstance(raw_inputs, list) or len(raw_inputs) != ARITY[operation]:
        raise DynamicAutogradValidationError(
            f"{context}.inputs: {operation} expects {ARITY[operation]} input(s)"
        )
    inputs = [
        _identifier(parent, f"{context}.inputs[{index}]")
        for index, parent in enumerate(raw_inputs)
    ]
    missing = [parent for parent in inputs if parent not in known]
    if missing:
        raise DynamicAutogradValidationError(
            f"{context}.inputs: parents must already exist: {missing}"
        )
    return {"id": step_id, "operation": operation, "inputs": inputs}


def _validate_case(value: Any, context: str) -> dict[str, Any]:
    case = _object(
        value,
        {
            "id",
            "title",
            "inputs",
            "steps",
            "output",
            "mutations_after_forward",
            "expected",
        },
        context,
    )
    raw_inputs = case["inputs"]
    if not isinstance(raw_inputs, list) or not 1 <= len(raw_inputs) <= MAX_INPUTS:
        raise DynamicAutogradValidationError(
            f"{context}.inputs: expected 1 to {MAX_INPUTS} inputs"
        )
    inputs = [
        _input(item, f"{context}.inputs[{index}]")
        for index, item in enumerate(raw_inputs)
    ]
    input_ids = [item["id"] for item in inputs]
    if len(set(input_ids)) != len(input_ids):
        raise DynamicAutogradValidationError(f"{context}.inputs: duplicate input id")

    raw_steps = case["steps"]
    if not isinstance(raw_steps, list) or not 1 <= len(raw_steps) <= MAX_STEPS:
        raise DynamicAutogradValidationError(
            f"{context}.steps: expected 1 to {MAX_STEPS} steps"
        )
    known = set(input_ids)
    steps = []
    for index, raw_step in enumerate(raw_steps):
        step = _step(raw_step, known, f"{context}.steps[{index}]")
        known.add(step["id"])
        steps.append(step)

    output = _identifier(case["output"], f"{context}.output")
    if output != steps[-1]["id"]:
        raise DynamicAutogradValidationError(
            f"{context}.output: expected the final executed step {steps[-1]['id']}"
        )
    raw_mutations = case["mutations_after_forward"]
    if not isinstance(raw_mutations, dict) or len(raw_mutations) > MAX_INPUTS:
        raise DynamicAutogradValidationError(
            f"{context}.mutations_after_forward: expected bounded object"
        )
    mutations: dict[str, float] = {}
    for raw_id, raw_value in raw_mutations.items():
        input_id = _identifier(raw_id, f"{context}.mutations_after_forward key")
        if input_id not in input_ids:
            raise DynamicAutogradValidationError(
                f"{context}.mutations_after_forward: unknown input {input_id}"
            )
        mutations[input_id] = _number(
            raw_value, f"{context}.mutations_after_forward.{input_id}"
        )
    if not isinstance(case["expected"], dict):
        raise DynamicAutogradValidationError(f"{context}.expected: expected object")
    return {
        "id": _identifier(case["id"], f"{context}.id"),
        "title": _text(case["title"], f"{context}.title"),
        "inputs": inputs,
        "steps": steps,
        "output": output,
        "mutations_after_forward": mutations,
        "expected": case["expected"],
    }


def _saved_value(node: dict[str, Any], name: str) -> float:
    for saved in node["saved"]:
        if saved["name"] == name:
            return saved["value"]
    raise DynamicAutogradValidationError(
        f"node {node['id']}: missing saved value {name}"
    )


def _forward_graph(
    case: dict[str, Any],
    overrides: dict[str, float] | None = None,
    *,
    allowed_input_headroom: float = 0.0,
) -> tuple[dict[str, dict[str, Any]], list[str], dict[str, str]]:
    values = overrides or {}
    nodes: dict[str, dict[str, Any]] = {}
    executed_node_ids: list[str] = []
    branch_choices: dict[str, str] = {}
    for item in case["inputs"]:
        value = _number(
            values.get(item["id"], item["value"]),
            f"input {item['id']}",
            bounded=False,
        )
        if abs(value) > MAX_ABSOLUTE_VALUE + allowed_input_headroom:
            raise DynamicAutogradValidationError(
                f"input {item['id']}: magnitude exceeds finite-difference headroom"
            )
        nodes[item["id"]] = {
            "id": item["id"],
            "operation": "input",
            "parents": [],
            "forward_value": value,
            "saved": [],
        }
        executed_node_ids.append(item["id"])

    for step in case["steps"]:
        parents = [nodes[parent_id] for parent_id in step["inputs"]]
        parent_values = [parent["forward_value"] for parent in parents]
        declared_operation = step["operation"]
        operation = declared_operation
        saved: list[dict[str, Any]] = []
        if declared_operation == "multiply":
            value = _finite(
                parent_values[0] * parent_values[1], f"{step['id']} multiply"
            )
            saved = [
                {
                    "name": "left",
                    "source_id": parents[0]["id"],
                    "value": parent_values[0],
                },
                {
                    "name": "right",
                    "source_id": parents[1]["id"],
                    "value": parent_values[1],
                },
            ]
        elif declared_operation == "add":
            value = _finite(parent_values[0] + parent_values[1], f"{step['id']} add")
        elif declared_operation == "square":
            value = _finite(parent_values[0] * parent_values[0], f"{step['id']} square")
            saved = [
                {
                    "name": "input",
                    "source_id": parents[0]["id"],
                    "value": parent_values[0],
                }
            ]
        elif declared_operation == "negate":
            value = _finite(-parent_values[0], f"{step['id']} negate")
        elif parent_values[0] >= 0:
            operation = "identity"
            branch_choices[step["id"]] = "nonnegative"
            value = parent_values[0]
        else:
            operation = "negate"
            branch_choices[step["id"]] = "negative"
            value = _finite(-parent_values[0], f"{step['id']} branch negate")
        nodes[step["id"]] = {
            "id": step["id"],
            "operation": operation,
            "parents": step["inputs"],
            "forward_value": value,
            "saved": saved,
        }
        executed_node_ids.append(step["id"])
    return nodes, executed_node_ids, branch_choices


def _topological_order(nodes: dict[str, dict[str, Any]], output: str) -> list[str]:
    order: list[str] = []
    visited: set[str] = set()

    def visit(node_id: str) -> None:
        if node_id in visited:
            return
        visited.add(node_id)
        for parent_id in nodes[node_id]["parents"]:
            visit(parent_id)
        order.append(node_id)

    visit(output)
    return order


def _local_derivatives(node: dict[str, Any]) -> list[dict[str, Any]]:
    parents = node["parents"]
    operation = node["operation"]
    if operation == "multiply":
        return [
            {
                "parent_id": parents[0],
                "value": _saved_value(node, "right"),
                "source": "saved:right",
            },
            {
                "parent_id": parents[1],
                "value": _saved_value(node, "left"),
                "source": "saved:left",
            },
        ]
    if operation == "add":
        return [
            {"parent_id": parents[0], "value": 1.0, "source": "constant:1"},
            {"parent_id": parents[1], "value": 1.0, "source": "constant:1"},
        ]
    if operation == "square":
        derivative = _finite(
            2 * _saved_value(node, "input"), f"{node['id']} derivative"
        )
        return [{"parent_id": parents[0], "value": derivative, "source": "saved:input"}]
    if operation == "negate":
        return [{"parent_id": parents[0], "value": -1.0, "source": "constant:-1"}]
    if operation == "identity":
        return [{"parent_id": parents[0], "value": 1.0, "source": "constant:1"}]
    raise DynamicAutogradValidationError(
        f"node {node['id']}: unsupported backward operation"
    )


def _forward_output(
    case: dict[str, Any], overrides: dict[str, float], finite_difference_epsilon: float
) -> float:
    nodes, _, _ = _forward_graph(
        case,
        overrides,
        allowed_input_headroom=finite_difference_epsilon,
    )
    return nodes[case["output"]]["forward_value"]


def execute_case(
    case: dict[str, Any],
    finite_difference_epsilon: float,
    *,
    apply_mutations: bool = True,
) -> dict[str, Any]:
    epsilon = _number(
        finite_difference_epsilon, "finite-difference epsilon", bounded=False
    )
    if epsilon < 1e-12 or epsilon > 1:
        raise DynamicAutogradValidationError(
            "finite-difference epsilon must be in [1e-12, 1]"
        )
    nodes, executed_node_ids, branch_choices = _forward_graph(case)
    topological_order = _topological_order(nodes, case["output"])
    backward_order = list(reversed(topological_order))
    gradients: dict[str, float] = {case["output"]: 1.0}
    backward_steps = []
    for node_id in backward_order:
        node = nodes[node_id]
        upstream = gradients.get(node_id)
        if upstream is None or node["operation"] == "input":
            continue
        derivatives = _local_derivatives(node)
        contributions = []
        for derivative in derivatives:
            contribution = _finite(
                upstream * derivative["value"], f"{node_id} parent contribution"
            )
            parent_id = derivative["parent_id"]
            gradients[parent_id] = _finite(
                gradients.get(parent_id, 0.0) + contribution,
                f"{parent_id} accumulated gradient",
            )
            contributions.append({"parent_id": parent_id, "value": contribution})
        backward_steps.append(
            {
                "node_id": node_id,
                "operation": node["operation"],
                "upstream_gradient": upstream,
                "local_derivatives": derivatives,
                "parent_contributions": contributions,
            }
        )

    finite_difference_gradients: dict[str, float] = {}
    errors = []
    base_inputs = {item["id"]: item["value"] for item in case["inputs"]}
    for item in case["inputs"]:
        input_id = item["id"]
        plus = dict(base_inputs)
        minus = dict(base_inputs)
        plus[input_id] = _finite(plus[input_id] + epsilon, f"{input_id} plus epsilon")
        minus[input_id] = _finite(
            minus[input_id] - epsilon, f"{input_id} minus epsilon"
        )
        numerical = _finite(
            (
                _forward_output(case, plus, epsilon)
                - _forward_output(case, minus, epsilon)
            )
            / (2 * epsilon),
            f"{input_id} finite difference",
        )
        finite_difference_gradients[input_id] = numerical
        errors.append(abs(gradients[input_id] - numerical))

    live_input_values = dict(base_inputs)
    if apply_mutations:
        live_input_values.update(case["mutations_after_forward"])
    return {
        "executed_node_ids": executed_node_ids,
        "executed_operations": {
            node_id: nodes[node_id]["operation"] for node_id in executed_node_ids
        },
        "topological_order": topological_order,
        "backward_order": backward_order,
        "forward_values": {
            node_id: nodes[node_id]["forward_value"] for node_id in executed_node_ids
        },
        "branch_choices": branch_choices,
        "saved_values": {
            step["id"]: nodes[step["id"]]["saved"] for step in case["steps"]
        },
        "live_input_values": live_input_values,
        "backward_steps": backward_steps,
        "gradients": gradients,
        "finite_difference_gradients": finite_difference_gradients,
        "max_gradient_absolute_error": max(errors, default=0.0),
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise DynamicAutogradValidationError(f"{context}: mismatch")
    elif isinstance(expected, (int, float)):
        expected_number = _number(expected, f"{context} expected value", bounded=False)
        if abs(_number(actual, context, bounded=False) - expected_number) > tolerance:
            raise DynamicAutogradValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
    elif isinstance(expected, str):
        if actual != expected:
            raise DynamicAutogradValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise DynamicAutogradValidationError(f"{context}: array length mismatch")
        for index, (left, right) in enumerate(zip(actual, expected)):
            _compare(left, right, tolerance, f"{context}[{index}]")
    elif isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            actual_keys = sorted(actual) if isinstance(actual, dict) else []
            raise DynamicAutogradValidationError(
                f"{context}: object keys expected {sorted(expected)}, got {actual_keys}"
            )
        for key, value in expected.items():
            _compare(actual[key], value, tolerance, f"{context}.{key}")
    else:
        raise DynamicAutogradValidationError(f"{context}: unsupported expected value")


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
        raise DynamicAutogradValidationError("lab.schema_version: expected 1")
    _text(lab["id"], "lab.id")
    _text(lab["title"], "lab.title")
    _text(lab["question"], "lab.question")
    tolerance = _number(
        lab["absolute_tolerance"], "lab.absolute_tolerance", bounded=False
    )
    if tolerance != CANONICAL_TOLERANCE:
        raise DynamicAutogradValidationError(
            f"lab.absolute_tolerance: expected canonical {CANONICAL_TOLERANCE}"
        )
    operation = _object(
        lab["operation"],
        {
            "kind",
            "graph_construction",
            "saved_value_policy",
            "backward_traversal",
            "finite_difference_epsilon",
        },
        "lab.operation",
    )
    expected_operation = {
        "kind": "dynamic-scalar-reverse-mode",
        "graph_construction": "executed-operations-only",
        "saved_value_policy": "immutable-forward-snapshots",
        "backward_traversal": "reverse-topological",
        "finite_difference_epsilon": CANONICAL_EPSILON,
    }
    _compare(operation, expected_operation, 0.0, "lab.operation")
    raw_cases = lab["cases"]
    if not isinstance(raw_cases, list) or len(raw_cases) != len(CASE_IDS):
        raise DynamicAutogradValidationError(
            f"lab.cases: expected {len(CASE_IDS)} cases"
        )
    cases = [
        _validate_case(case, f"lab.cases[{index}]")
        for index, case in enumerate(raw_cases)
    ]
    case_ids = [case["id"] for case in cases]
    if case_ids != CASE_IDS:
        raise DynamicAutogradValidationError(
            f"lab.cases: case ids expected {CASE_IDS}, got {case_ids}"
        )
    for case in cases:
        trace = execute_case(case, CANONICAL_EPSILON)
        if trace["max_gradient_absolute_error"] > tolerance:
            raise DynamicAutogradValidationError(
                f"case {case['id']}: numerical gradient error "
                f"{trace['max_gradient_absolute_error']!r} exceeds {tolerance!r}"
            )
        _compare(trace, case["expected"], tolerance, f"case {case['id']}.expected")
    return {**lab, "cases": cases}


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    paths = sorted((root / "labs").glob("*.json"))
    if not paths:
        raise DynamicAutogradValidationError(f"{root}: no lab JSON files")
    for path in paths:
        validate_document(load_json(path))
    return len(paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    try:
        count = validate_fixture_root(args.root)
    except DynamicAutogradValidationError as error:
        parser.error(str(error))
    print(f"validated {count} dynamic autograd lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
