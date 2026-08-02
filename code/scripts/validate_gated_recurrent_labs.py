#!/usr/bin/env python3
"""Validate and execute the deterministic NN11 gated recurrent corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "gated-recurrent-v1"


class GatedRecurrentValidationError(ValueError):
    """Raised when an NN11 document or deterministic result is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GatedRecurrentValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                GatedRecurrentValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise GatedRecurrentValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise GatedRecurrentValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: Any, required: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GatedRecurrentValidationError(f"{context}: expected an object")
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise GatedRecurrentValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise GatedRecurrentValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise GatedRecurrentValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise GatedRecurrentValidationError(f"{context}: expected a finite number")
    return result


def _sigmoid(value: float) -> float:
    if value >= 0:
        factor = math.exp(-value)
        return 1.0 / (1.0 + factor)
    factor = math.exp(value)
    return factor / (1.0 + factor)


def _gate(value: Any, context: str, activation: str) -> dict[str, float]:
    gate = _require_keys(value, {"preactivation", "value"}, context)
    preactivation = _number(gate["preactivation"], f"{context}.preactivation")
    actual = (
        _sigmoid(preactivation) if activation == "sigmoid" else math.tanh(preactivation)
    )
    return {"preactivation": preactivation, "value": actual}


def _compare(expected: Any, actual: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected != actual:
            raise GatedRecurrentValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
        return
    if isinstance(expected, (int, float)) and isinstance(actual, (int, float)):
        if not math.isclose(
            float(expected), float(actual), rel_tol=0, abs_tol=tolerance
        ):
            raise GatedRecurrentValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
        return
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            raise GatedRecurrentValidationError(
                f"{context}: expected {len(expected)} items, got {len(actual)}"
            )
        for index, (expected_item, actual_item) in enumerate(zip(expected, actual)):
            _compare(expected_item, actual_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            raise GatedRecurrentValidationError(
                f"{context}: key mismatch {sorted(expected)} != {sorted(actual)}"
            )
        for key in expected:
            _compare(expected[key], actual[key], tolerance, f"{context}.{key}")
        return
    if expected != actual:
        raise GatedRecurrentValidationError(
            f"{context}: expected {expected!r}, got {actual!r}"
        )


def _execute_gru(
    gru: dict[str, Any], input_value: float, previous_hidden: float
) -> dict[str, Any]:
    reset_gate = _gate(gru["reset_gate"], "gru.reset_gate", "sigmoid")
    update_gate = _gate(gru["update_gate"], "gru.update_gate", "sigmoid")
    candidate = _require_keys(
        gru["candidate"],
        {
            "input_weight",
            "input_product",
            "reset_state",
            "recurrent_weight",
            "recurrent_product",
            "bias",
            "preactivation",
            "value",
        },
        "gru.candidate",
    )
    input_weight = _number(candidate["input_weight"], "gru.candidate.input_weight")
    recurrent_weight = _number(
        candidate["recurrent_weight"], "gru.candidate.recurrent_weight"
    )
    bias = _number(candidate["bias"], "gru.candidate.bias")
    input_product = input_weight * input_value
    reset_state = reset_gate["value"] * previous_hidden
    recurrent_product = recurrent_weight * reset_state
    preactivation = input_product + recurrent_product + bias
    candidate_value = math.tanh(preactivation)
    retained_state = (1.0 - update_gate["value"]) * previous_hidden
    candidate_write = update_gate["value"] * candidate_value
    hidden_state = retained_state + candidate_write
    return {
        "reset_gate": reset_gate,
        "update_gate": update_gate,
        "candidate": {
            "input_weight": input_weight,
            "input_product": input_product,
            "reset_state": reset_state,
            "recurrent_weight": recurrent_weight,
            "recurrent_product": recurrent_product,
            "bias": bias,
            "preactivation": preactivation,
            "value": candidate_value,
        },
        "output": {
            "retained_state": retained_state,
            "candidate_write": candidate_write,
            "hidden_state": hidden_state,
        },
    }


def _execute_lstm(lstm: dict[str, Any], previous_cell: float) -> dict[str, Any]:
    forget_gate = _gate(lstm["forget_gate"], "lstm.forget_gate", "sigmoid")
    input_gate = _gate(lstm["input_gate"], "lstm.input_gate", "sigmoid")
    output_gate = _gate(lstm["output_gate"], "lstm.output_gate", "sigmoid")
    candidate = _gate(lstm["candidate"], "lstm.candidate", "tanh")
    retained_cell = forget_gate["value"] * previous_cell
    candidate_write = input_gate["value"] * candidate["value"]
    cell_state = retained_cell + candidate_write
    exposed_cell = math.tanh(cell_state)
    hidden_state = output_gate["value"] * exposed_cell
    return {
        "forget_gate": forget_gate,
        "input_gate": input_gate,
        "output_gate": output_gate,
        "candidate": candidate,
        "output": {
            "retained_cell": retained_cell,
            "candidate_write": candidate_write,
            "cell_state": cell_state,
            "exposed_cell": exposed_cell,
            "hidden_state": hidden_state,
        },
    }


def _counterfactuals(
    gru: dict[str, Any],
    lstm: dict[str, Any],
    previous_hidden: float,
    previous_cell: float,
) -> dict[str, list[dict[str, Any]]]:
    gru_candidate = gru["candidate"]
    update = gru["update_gate"]["value"]
    reset_zero_candidate = math.tanh(
        gru_candidate["input_product"] + gru_candidate["bias"]
    )
    canonical_candidate = gru_candidate["value"]
    gru_rows = [
        {
            "gate": "update",
            "gate_value": 0.0,
            "candidate": canonical_candidate,
            "cell_state": None,
            "hidden_state": previous_hidden,
        },
        {
            "gate": "update",
            "gate_value": 1.0,
            "candidate": canonical_candidate,
            "cell_state": None,
            "hidden_state": canonical_candidate,
        },
        {
            "gate": "reset",
            "gate_value": 0.0,
            "candidate": reset_zero_candidate,
            "cell_state": None,
            "hidden_state": (1.0 - update) * previous_hidden
            + update * reset_zero_candidate,
        },
    ]
    forget = lstm["forget_gate"]["value"]
    input_gate = lstm["input_gate"]["value"]
    output = lstm["output_gate"]["value"]
    candidate = lstm["candidate"]["value"]

    def lstm_row(gate: str, gate_value: float) -> dict[str, Any]:
        row_forget = gate_value if gate == "forget" else forget
        row_input = gate_value if gate == "input" else input_gate
        row_output = gate_value if gate == "output" else output
        cell = row_forget * previous_cell + row_input * candidate
        return {
            "gate": gate,
            "gate_value": gate_value,
            "candidate": candidate,
            "cell_state": cell,
            "hidden_state": row_output * math.tanh(cell),
        }

    return {
        "gru": gru_rows,
        "lstm": [
            lstm_row("forget", 0.0),
            lstm_row("input", 0.0),
            lstm_row("output", 0.0),
            lstm_row("output", 1.0),
        ],
    }


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    inputs = _require_keys(
        document["shared_inputs"],
        {"input", "previous_hidden", "previous_cell"},
        "shared_inputs",
    )
    input_value = _number(inputs["input"], "shared_inputs.input")
    previous_hidden = _number(
        inputs["previous_hidden"], "shared_inputs.previous_hidden"
    )
    previous_cell = _number(inputs["previous_cell"], "shared_inputs.previous_cell")
    gru = _execute_gru(document["gru"], input_value, previous_hidden)
    lstm = _execute_lstm(document["lstm"], previous_cell)
    return {
        "gru": gru,
        "lstm": lstm,
        "counterfactuals": _counterfactuals(gru, lstm, previous_hidden, previous_cell),
    }


def validate_lab(document: dict[str, Any]) -> dict[str, Any]:
    _require_keys(
        document,
        {
            "schema_version",
            "id",
            "title",
            "question",
            "absolute_tolerance",
            "concepts",
            "operation",
            "shared_inputs",
            "gru",
            "lstm",
            "counterfactuals",
        },
        "lab",
    )
    if document["schema_version"] != 1:
        raise GatedRecurrentValidationError("lab.schema_version: expected 1")
    for key in ("id", "title", "question"):
        if not isinstance(document[key], str) or not document[key]:
            raise GatedRecurrentValidationError(f"lab.{key}: expected text")
    concepts = document["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise GatedRecurrentValidationError("lab.concepts: expected unique text")
    tolerance = _number(document["absolute_tolerance"], "lab.absolute_tolerance")
    if tolerance <= 0:
        raise GatedRecurrentValidationError(
            "lab.absolute_tolerance: expected a positive number"
        )
    operation = _require_keys(
        document["operation"],
        {
            "kind",
            "gru_update_convention",
            "gate_activation",
            "candidate_activation",
        },
        "operation",
    )
    expected_operation = {
        "kind": "scalar-gru-lstm-gate-comparison",
        "gru_update_convention": "candidate-share",
        "gate_activation": "sigmoid",
        "candidate_activation": "tanh",
    }
    if operation != expected_operation:
        raise GatedRecurrentValidationError(
            f"operation: expected {expected_operation!r}"
        )
    _require_keys(
        document["gru"],
        {"reset_gate", "update_gate", "candidate", "output"},
        "gru",
    )
    _require_keys(
        document["lstm"],
        {"forget_gate", "input_gate", "output_gate", "candidate", "output"},
        "lstm",
    )
    actual = execute_lab(document)
    _compare(document["gru"], actual["gru"], tolerance, "gru")
    _compare(document["lstm"], actual["lstm"], tolerance, "lstm")
    _compare(
        document["counterfactuals"],
        actual["counterfactuals"],
        tolerance,
        "counterfactuals",
    )
    return actual


def validate_corpus(root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    lab_paths = sorted((root / "labs").glob("*.json"))
    if not lab_paths:
        raise GatedRecurrentValidationError(f"no labs found under {root / 'labs'}")
    for path in lab_paths:
        try:
            validate_lab(load_json(path))
        except GatedRecurrentValidationError as error:
            raise GatedRecurrentValidationError(f"{path}: {error}") from error
    return lab_paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    paths = validate_corpus(args.root)
    print(f"validated {len(paths)} gated recurrent labs from {args.root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
