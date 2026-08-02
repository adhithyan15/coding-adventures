#!/usr/bin/env python3
"""Validate the deterministic NN30 backward/optimizer lowering corpus."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "backward-optimizer-lowering-v1"
)

CANONICAL_ID = "backward-optimizer-lowering"
CANONICAL_TOLERANCE = 1e-8
CANONICAL_EPSILON = 1e-5
CANONICAL_SCENARIOS = ["one_row_by_hand", "two_row_mean", "persistent_buffer"]
MAX_SCENARIOS = 4
MAX_BATCH = 8
MAX_INSTRUCTIONS = 16
MAX_TEXT_LENGTH = 512
MAX_IDENTIFIER_LENGTH = 64
MAX_ABSOLUTE_INPUT = 1e3
MAX_ABSOLUTE_DERIVED = 1e12
MAX_COMPARE_DEPTH = 64
IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]*$")


class BackwardOptimizerLoweringValidationError(ValueError):
    """Raised when an NN30 fixture violates the executable contract."""


def _duplicate_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise BackwardOptimizerLoweringValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    raise BackwardOptimizerLoweringValidationError(f"non-finite JSON constant: {value}")


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_duplicate_object,
            parse_constant=_reject_constant,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise BackwardOptimizerLoweringValidationError(f"{path}: {error}") from error
    if not isinstance(value, dict):
        raise BackwardOptimizerLoweringValidationError(f"{path}: expected JSON object")
    return value


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise BackwardOptimizerLoweringValidationError(f"{context}: expected object")
    if set(value) != keys:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: key mismatch, expected {sorted(keys)}, got {sorted(value)}"
        )
    return value


def _text(value: Any, context: str) -> str:
    if not isinstance(value, str) or not 1 <= len(value) <= MAX_TEXT_LENGTH:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected non-empty string up to {MAX_TEXT_LENGTH} characters"
        )
    return value


def _identifier(value: Any, context: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) > MAX_IDENTIFIER_LENGTH
        or IDENTIFIER.fullmatch(value) is None
    ):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected canonical identifier"
        )
    return value


def _number(value: Any, context: str, *, derived: bool = False) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise BackwardOptimizerLoweringValidationError(f"{context}: expected number")
    limit = MAX_ABSOLUTE_DERIVED if derived else MAX_ABSOLUTE_INPUT
    if abs(value) > limit:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected finite bounded number with abs <= {limit:g}"
        )
    if isinstance(value, float) and not math.isfinite(value):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected finite bounded number"
        )
    return float(value)


def _finite(value: float, context: str) -> float:
    if not math.isfinite(value) or abs(value) > MAX_ABSOLUTE_DERIVED:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: derived value must be finite with abs <= {MAX_ABSOLUTE_DERIVED:g}"
        )
    return value


def _instruction(
    instruction_id: str,
    op: str,
    output: str,
    inputs: list[str],
    *,
    attributes: dict[str, Any] | None = None,
    source_nodes: list[str] | None = None,
    source_edges: list[str] | None = None,
    source_instructions: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "id": instruction_id,
        "op": op,
        "output": output,
        "inputs": inputs,
        "attributes": attributes or {},
        "source_nodes": source_nodes or [],
        "source_edges": source_edges or [],
        "source_instructions": source_instructions or [],
    }


def compile_backward_ir() -> dict[str, Any]:
    return {
        "magic": "CANB",
        "version": 0,
        "instructions": [
            _instruction(
                "b0",
                "SEED_LOSS_GRAD",
                "d_loss",
                [],
                attributes={"value": 1},
                source_nodes=["loss"],
            ),
            _instruction(
                "b1",
                "HALF_SQUARED_ERROR_GRAD",
                "d_residual",
                ["residual", "d_loss"],
                source_nodes=["loss", "residual"],
            ),
            _instruction(
                "b2",
                "PROPAGATE_GRAD",
                "d_prediction",
                ["d_residual"],
                attributes={"through": "subtract_prediction"},
                source_nodes=["residual", "prediction"],
            ),
            _instruction(
                "b3",
                "PARAMETER_LOCAL_GRAD",
                "local_d_w",
                ["x", "d_prediction"],
                attributes={"parameter_id": "w"},
                source_nodes=["prediction"],
                source_edges=["w"],
            ),
            _instruction(
                "b4",
                "ACCUMULATE_GRAD",
                "grad_w",
                ["grad_w", "local_d_w"],
                attributes={"parameter_id": "w", "order": "row_ascending"},
                source_edges=["w"],
            ),
            _instruction(
                "b5",
                "INPUT_GRAD",
                "d_x",
                ["w", "d_prediction"],
                attributes={"input_id": "x"},
                source_nodes=["x", "prediction"],
                source_edges=["w"],
            ),
        ],
    }


def compile_optimizer_ir() -> dict[str, Any]:
    return {
        "magic": "CANO",
        "version": 0,
        "instructions": [
            _instruction(
                "o0",
                "READ_GRAD_BUFFER",
                "total_d_w",
                ["grad_w"],
                attributes={"parameter_id": "w"},
                source_edges=["w"],
            ),
            _instruction(
                "o1",
                "DIVIDE_GRAD",
                "applied_d_w",
                ["total_d_w"],
                attributes={"divisor_source": "scenario.divisor"},
                source_edges=["w"],
                source_instructions=["o0"],
            ),
            _instruction(
                "o2",
                "SGD_UPDATE",
                "w_next",
                ["w", "applied_d_w"],
                attributes={"learning_rate_source": "scenario.learning_rate"},
                source_edges=["w"],
                source_instructions=["o1"],
            ),
            _instruction(
                "o3",
                "KEEP_GRAD_BUFFER",
                "grad_w_after_step",
                ["grad_w"],
                attributes={"optimizer_step_zeroes_gradient": False},
                source_edges=["w"],
                source_instructions=["o2"],
            ),
        ],
    }


def compile_matrix_training_ir() -> dict[str, Any]:
    return {
        "magic": "CANM-TRAIN",
        "version": 0,
        "instructions": [
            _instruction(
                "t0",
                "LOAD_SAVED_COLUMN",
                "x_col",
                ["x"],
                attributes={"saved_value": "x"},
                source_nodes=["x"],
            ),
            _instruction(
                "t1",
                "LOAD_SAVED_COLUMN",
                "residual_col",
                ["residual"],
                attributes={"saved_value": "residual"},
                source_nodes=["residual"],
            ),
            _instruction(
                "t2",
                "LOSS_GRAD_COLUMN",
                "d_prediction_col",
                ["residual_col"],
                attributes={"loss": "half_squared_error"},
                source_nodes=["loss", "prediction"],
                source_instructions=["b0", "b1", "b2"],
            ),
            _instruction(
                "t3",
                "PARAMETER_LOCAL_GRAD_COLUMN",
                "local_d_w_col",
                ["x_col", "d_prediction_col"],
                attributes={"parameter_id": "w"},
                source_nodes=["prediction"],
                source_edges=["w"],
                source_instructions=["b3"],
            ),
            _instruction(
                "t4",
                "INPUT_GRAD_COLUMN",
                "d_x_col",
                ["d_prediction_col"],
                attributes={"input_id": "x", "parameter_id": "w"},
                source_nodes=["x", "prediction"],
                source_edges=["w"],
                source_instructions=["b5"],
            ),
            _instruction(
                "t5",
                "REDUCE_SUM_GRAD",
                "batch_d_w",
                ["local_d_w_col"],
                attributes={"order": "row_ascending", "parameter_id": "w"},
                source_edges=["w"],
                source_instructions=["b4"],
            ),
            _instruction(
                "t6",
                "ACCUMULATE_GRAD_BUFFER",
                "grad_w",
                ["grad_w", "batch_d_w"],
                attributes={"parameter_id": "w"},
                source_edges=["w"],
                source_instructions=["b4"],
            ),
            _instruction(
                "t7",
                "DIVIDE_GRAD",
                "applied_d_w",
                ["grad_w"],
                attributes={"divisor_source": "scenario.divisor"},
                source_edges=["w"],
                source_instructions=["o0", "o1"],
            ),
            _instruction(
                "t8",
                "SGD_UPDATE_SCALAR",
                "w_next",
                ["w", "applied_d_w"],
                attributes={"learning_rate_source": "scenario.learning_rate"},
                source_edges=["w"],
                source_instructions=["o2"],
            ),
            _instruction(
                "t9",
                "KEEP_GRAD_BUFFER",
                "grad_w_after_step",
                ["grad_w"],
                attributes={"optimizer_step_zeroes_gradient": False},
                source_edges=["w"],
                source_instructions=["o3"],
            ),
        ],
    }


def compile_training_ir() -> dict[str, Any]:
    return {
        "backward": compile_backward_ir(),
        "optimizer": compile_optimizer_ir(),
        "matrix_training": compile_matrix_training_ir(),
    }


def _number_array(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not 1 <= len(value) <= MAX_BATCH:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected 1 to {MAX_BATCH} numbers"
        )
    return [_number(item, f"{context}[{index}]") for index, item in enumerate(value)]


def _loss_sum(parameter: float, inputs: list[float], targets: list[float]) -> float:
    total = 0.0
    for index, (input_value, target) in enumerate(zip(inputs, targets, strict=True)):
        prediction = _finite(parameter * input_value, f"audit row {index} prediction")
        residual = _finite(prediction - target, f"audit row {index} residual")
        total = _finite(total + 0.5 * residual * residual, "audit loss sum")
    return total


def execute_scenario(scenario: dict[str, Any], epsilon: float) -> dict[str, Any]:
    parameter = scenario["initial_parameter"]
    learning_rate = scenario["learning_rate"]
    inputs = scenario["inputs"]
    targets = scenario["targets"]
    gradient_buffer_before = scenario["gradient_buffer_before"]
    divisor = scenario["divisor"]

    predictions: list[float] = []
    residuals: list[float] = []
    losses: list[float] = []
    d_loss: list[float] = []
    d_residual: list[float] = []
    d_prediction: list[float] = []
    local_d_w: list[float] = []
    d_x: list[float] = []
    batch_gradient = 0.0

    for index, (input_value, target) in enumerate(zip(inputs, targets, strict=True)):
        prediction = _finite(parameter * input_value, f"row {index} prediction")
        residual = _finite(prediction - target, f"row {index} residual")
        loss = _finite(0.5 * residual * residual, f"row {index} loss")
        loss_seed = 1.0
        residual_grad = _finite(residual * loss_seed, f"row {index} d_residual")
        prediction_grad = residual_grad
        parameter_grad = _finite(
            input_value * prediction_grad, f"row {index} local_d_w"
        )
        input_grad = _finite(parameter * prediction_grad, f"row {index} d_x")
        batch_gradient = _finite(
            batch_gradient + parameter_grad, "batch gradient stable row reduction"
        )
        predictions.append(prediction)
        residuals.append(residual)
        losses.append(loss)
        d_loss.append(loss_seed)
        d_residual.append(residual_grad)
        d_prediction.append(prediction_grad)
        local_d_w.append(parameter_grad)
        d_x.append(input_grad)

    grad_w = _finite(
        gradient_buffer_before + batch_gradient, "persistent grad_w accumulation"
    )
    applied_gradient = _finite(grad_w / divisor, "applied_gradient")
    parameter_delta = _finite(-learning_rate * applied_gradient, "parameter_delta")
    parameter_after = _finite(parameter + parameter_delta, "parameter_after")
    numerical_gradient = _finite(
        (
            _loss_sum(parameter + epsilon, inputs, targets)
            - _loss_sum(parameter - epsilon, inputs, targets)
        )
        / (2 * epsilon),
        "numerical_gradient",
    )
    gradient_error = _finite(abs(batch_gradient - numerical_gradient), "gradient_error")

    saved_values = {
        "x": list(inputs),
        "target": list(targets),
        "prediction": predictions,
        "residual": residuals,
        "loss": losses,
    }
    backward = {
        "d_loss": d_loss,
        "d_residual": d_residual,
        "d_prediction": d_prediction,
        "local_d_w": local_d_w,
        "d_x": d_x,
        "gradient_buffer_before": gradient_buffer_before,
        "batch_gradient": batch_gradient,
        "grad_w": grad_w,
    }
    optimizer = {
        "parameter_before": parameter,
        "applied_gradient": applied_gradient,
        "parameter_delta": parameter_delta,
        "parameter_after": parameter_after,
        "gradient_buffer_after_step": grad_w,
    }
    matrix_training = {
        "columns": {
            "x": list(inputs),
            "residual": list(residuals),
            "d_prediction": list(d_prediction),
            "local_d_w": list(local_d_w),
            "d_x": list(d_x),
        },
        "gradient_buffer_before": gradient_buffer_before,
        "batch_gradient": batch_gradient,
        "grad_w": grad_w,
        "applied_gradient": applied_gradient,
        "parameter_after": parameter_after,
        "gradient_buffer_after_step": grad_w,
    }
    path_values = [
        abs(backward["batch_gradient"] - matrix_training["batch_gradient"]),
        abs(backward["grad_w"] - matrix_training["grad_w"]),
        abs(optimizer["applied_gradient"] - matrix_training["applied_gradient"]),
        abs(optimizer["parameter_after"] - matrix_training["parameter_after"]),
        abs(
            optimizer["gradient_buffer_after_step"]
            - matrix_training["gradient_buffer_after_step"]
        ),
    ]
    return {
        "saved_values": saved_values,
        "backward": backward,
        "optimizer": optimizer,
        "matrix_training": matrix_training,
        "gradient_audit": {
            "analytical": batch_gradient,
            "numerical": numerical_gradient,
            "absolute_error": gradient_error,
        },
        "max_path_error": max(path_values),
    }


def _compare(
    actual: Any,
    expected: Any,
    tolerance: float,
    context: str,
    depth: int = 0,
) -> None:
    if depth > MAX_COMPARE_DEPTH:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: comparison nesting exceeds {MAX_COMPARE_DEPTH}"
        )
    if isinstance(expected, dict):
        if not isinstance(actual, dict) or set(actual) != set(expected):
            raise BackwardOptimizerLoweringValidationError(
                f"{context}: object key mismatch"
            )
        for key, expected_value in expected.items():
            _compare(
                actual[key],
                expected_value,
                tolerance,
                f"{context}.{key}",
                depth + 1,
            )
        return
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise BackwardOptimizerLoweringValidationError(
                f"{context}: list length mismatch"
            )
        for index, (left, right) in enumerate(zip(actual, expected, strict=True)):
            _compare(left, right, tolerance, f"{context}[{index}]", depth + 1)
        return
    if isinstance(expected, bool) or expected is None or isinstance(expected, str):
        if actual != expected or type(actual) is not type(expected):
            raise BackwardOptimizerLoweringValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
        return
    expected_number = _number(expected, context, derived=True)
    actual_number = _number(actual, context, derived=True)
    if not math.isclose(actual_number, expected_number, rel_tol=0.0, abs_tol=tolerance):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: expected {expected_number:g}, got {actual_number:g}"
        )


def _validate_training_graph(value: Any) -> dict[str, Any]:
    keys = {
        "equation",
        "loss",
        "parameter_id",
        "input_id",
        "target_id",
        "gradient_reduction",
        "optimizer",
        "optimizer_step_zeroes_gradient",
    }
    graph = _object(value, keys, "lab.training_graph")
    canonical = {
        "equation": "prediction = w * x",
        "loss": "half_squared_error",
        "parameter_id": "w",
        "input_id": "x",
        "target_id": "target",
        "gradient_reduction": "stable_row_sum_then_explicit_divisor",
        "optimizer": "sgd",
        "optimizer_step_zeroes_gradient": False,
    }
    if graph != canonical:
        raise BackwardOptimizerLoweringValidationError(
            "lab.training_graph: expected canonical scalar training graph"
        )
    return dict(graph)


def _validate_scenario(value: Any, context: str) -> dict[str, Any]:
    scenario = _object(
        value,
        {
            "id",
            "title",
            "initial_parameter",
            "learning_rate",
            "inputs",
            "targets",
            "gradient_buffer_before",
            "divisor",
            "expected",
        },
        context,
    )
    scenario_id = _identifier(scenario["id"], f"{context}.id")
    title = _text(scenario["title"], f"{context}.title")
    initial_parameter = _number(
        scenario["initial_parameter"], f"{context}.initial_parameter"
    )
    learning_rate = _number(scenario["learning_rate"], f"{context}.learning_rate")
    if learning_rate <= 0:
        raise BackwardOptimizerLoweringValidationError(
            f"{context}.learning_rate: expected positive value"
        )
    inputs = _number_array(scenario["inputs"], f"{context}.inputs")
    targets = _number_array(scenario["targets"], f"{context}.targets")
    gradient_buffer_before = _number(
        scenario["gradient_buffer_before"], f"{context}.gradient_buffer_before"
    )
    if len(inputs) != len(targets):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}: inputs and targets must have the same length"
        )
    divisor = scenario["divisor"]
    if (
        isinstance(divisor, bool)
        or not isinstance(divisor, int)
        or not 1 <= divisor <= len(inputs)
    ):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}.divisor: expected integer from 1 through batch length"
        )
    if not isinstance(scenario["expected"], dict):
        raise BackwardOptimizerLoweringValidationError(
            f"{context}.expected: expected object"
        )
    return {
        "id": scenario_id,
        "title": title,
        "initial_parameter": initial_parameter,
        "learning_rate": learning_rate,
        "inputs": inputs,
        "targets": targets,
        "gradient_buffer_before": gradient_buffer_before,
        "divisor": divisor,
        "expected": scenario["expected"],
    }


def validate_document(value: Any) -> dict[str, Any]:
    lab = _object(
        value,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "finite_difference_epsilon",
            "training_graph",
            "expected_ir",
            "scenarios",
        },
        "lab",
    )
    if lab["schema_version"] != 1 or isinstance(lab["schema_version"], bool):
        raise BackwardOptimizerLoweringValidationError("lab.schema_version: expected 1")
    if lab["id"] != CANONICAL_ID:
        raise BackwardOptimizerLoweringValidationError(
            f"lab.id: expected {CANONICAL_ID}"
        )
    title = _text(lab["title"], "lab.title")
    question = _text(lab["question"], "lab.question")
    tolerance = _number(lab["absolute_tolerance"], "lab.absolute_tolerance")
    epsilon = _number(lab["finite_difference_epsilon"], "lab.finite_difference_epsilon")
    if tolerance != CANONICAL_TOLERANCE:
        raise BackwardOptimizerLoweringValidationError(
            f"lab.absolute_tolerance: expected canonical {CANONICAL_TOLERANCE:g}"
        )
    if epsilon != CANONICAL_EPSILON:
        raise BackwardOptimizerLoweringValidationError(
            f"lab.finite_difference_epsilon: expected canonical {CANONICAL_EPSILON:g}"
        )
    graph = _validate_training_graph(lab["training_graph"])
    compiled_ir = compile_training_ir()
    _compare(compiled_ir, lab["expected_ir"], 0.0, "lab.expected_ir")
    for stream_name, stream in compiled_ir.items():
        if len(stream["instructions"]) > MAX_INSTRUCTIONS:
            raise BackwardOptimizerLoweringValidationError(
                f"{stream_name}: exceeds {MAX_INSTRUCTIONS} instructions"
            )

    raw_scenarios = lab["scenarios"]
    if (
        not isinstance(raw_scenarios, list)
        or not 1 <= len(raw_scenarios) <= MAX_SCENARIOS
    ):
        raise BackwardOptimizerLoweringValidationError(
            f"lab.scenarios: expected 1 to {MAX_SCENARIOS} scenarios"
        )
    scenarios = [
        _validate_scenario(item, f"lab.scenarios[{index}]")
        for index, item in enumerate(raw_scenarios)
    ]
    scenario_ids = [scenario["id"] for scenario in scenarios]
    if scenario_ids != CANONICAL_SCENARIOS:
        raise BackwardOptimizerLoweringValidationError(
            f"lab.scenarios: ids expected {CANONICAL_SCENARIOS}, got {scenario_ids}"
        )
    for scenario in scenarios:
        trace = execute_scenario(scenario, epsilon)
        _compare(
            trace,
            scenario["expected"],
            tolerance,
            f"scenario {scenario['id']}.expected",
        )
        if trace["gradient_audit"]["absolute_error"] > tolerance:
            raise BackwardOptimizerLoweringValidationError(
                f"scenario {scenario['id']}: numerical gradient audit exceeds tolerance"
            )
        if trace["max_path_error"] > tolerance:
            raise BackwardOptimizerLoweringValidationError(
                f"scenario {scenario['id']}: execution paths diverge"
            )

    return {
        "schema_version": 1,
        "id": CANONICAL_ID,
        "title": title,
        "question": question,
        "absolute_tolerance": tolerance,
        "finite_difference_epsilon": epsilon,
        "training_graph": graph,
        "expected_ir": compiled_ir,
        "scenarios": scenarios,
    }


def validate_fixture_root(root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    lab_paths = sorted((root / "labs").glob("*.json"))
    if not lab_paths:
        raise BackwardOptimizerLoweringValidationError(f"{root}: no lab JSON files")
    if len(lab_paths) > 8:
        raise BackwardOptimizerLoweringValidationError(f"{root}: too many lab files")
    for path in lab_paths:
        validate_document(load_json(path))
    return len(lab_paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "fixture_root",
        nargs="?",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )
    args = parser.parse_args()
    count = validate_fixture_root(args.fixture_root)
    print(f"validated {count} backward/optimizer lowering lab(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
