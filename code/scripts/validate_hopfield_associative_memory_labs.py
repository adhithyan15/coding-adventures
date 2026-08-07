#!/usr/bin/env python3
"""Validate and execute the deterministic NN20 Hopfield memory corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "hopfield-associative-memory-v1"
)


class HopfieldAssociativeMemoryValidationError(ValueError):
    """Raised when an NN20 document or deterministic trace is invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise HopfieldAssociativeMemoryValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                HopfieldAssociativeMemoryValidationError(
                    f"non-finite JSON number: {value}"
                )
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise HopfieldAssociativeMemoryValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise HopfieldAssociativeMemoryValidationError(
            "top-level JSON value must be an object"
        )
    return document


def _object(value: Any, keys: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise HopfieldAssociativeMemoryValidationError(f"{context}: expected an object")
    missing = keys - value.keys()
    extra = value.keys() - keys
    if missing or extra:
        raise HopfieldAssociativeMemoryValidationError(
            f"{context}: key mismatch missing={sorted(missing)} extra={sorted(extra)}"
        )
    return value


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise HopfieldAssociativeMemoryValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise HopfieldAssociativeMemoryValidationError(
            f"{context}: expected a finite number"
        )
    return result


def _integer(value: Any, context: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise HopfieldAssociativeMemoryValidationError(
            f"{context}: expected an integer"
        )
    return value


def _bipolar_vector(value: Any, context: str) -> list[int]:
    if not isinstance(value, list) or len(value) < 2:
        raise HopfieldAssociativeMemoryValidationError(
            f"{context}: expected at least two bipolar values"
        )
    result = [_integer(item, f"{context}[{index}]") for index, item in enumerate(value)]
    if any(item not in (-1, 1) for item in result):
        raise HopfieldAssociativeMemoryValidationError(
            f"{context}: values must be bipolar (-1 or 1)"
        )
    return result


def hopfield_weights(pattern: list[int]) -> list[list[float]]:
    size = len(pattern)
    return [
        [
            0.0 if row == column else pattern[row] * pattern[column] / size
            for column in range(size)
        ]
        for row in range(size)
    ]


def hopfield_energy(state: list[int], weights: list[list[float]]) -> float:
    directed_sum = sum(
        weights[row][column] * state[row] * state[column]
        for row in range(len(state))
        for column in range(len(state))
    )
    return -0.5 * directed_sum


def normalized_overlap(pattern: list[int], state: list[int]) -> float:
    return sum(saved * current for saved, current in zip(pattern, state)) / len(pattern)


def hamming_distance(pattern: list[int], state: list[int]) -> int:
    return sum(saved != current for saved, current in zip(pattern, state))


def execute_lab(document: dict[str, Any]) -> dict[str, Any]:
    pattern = _bipolar_vector(document["stored_pattern"], "stored_pattern")
    corrupted = _bipolar_vector(document["corrupted_state"], "corrupted_state")
    if len(corrupted) != len(pattern):
        raise HopfieldAssociativeMemoryValidationError(
            "stored_pattern and corrupted_state must have equal length"
        )

    order_value = document["update_order"]
    if not isinstance(order_value, list):
        raise HopfieldAssociativeMemoryValidationError(
            "update_order: expected an array"
        )
    order = [
        _integer(item, f"update_order[{index}]")
        for index, item in enumerate(order_value)
    ]
    if sorted(order) != list(range(len(pattern))):
        raise HopfieldAssociativeMemoryValidationError(
            "update_order must be one permutation of every neuron index"
        )

    weights = hopfield_weights(pattern)
    state = corrupted[:]
    updates: list[dict[str, Any]] = []
    for step, neuron_index in enumerate(order):
        state_before = state[:]
        incoming = [
            {
                "source_index": source_index,
                "weight": weights[neuron_index][source_index],
                "source_state": source_state,
                "contribution": weights[neuron_index][source_index] * source_state,
            }
            for source_index, source_state in enumerate(state_before)
        ]
        local_field = sum(row["contribution"] for row in incoming)
        previous_state = state_before[neuron_index]
        next_state = 1 if local_field > 0 else -1 if local_field < 0 else previous_state
        state = state_before[:]
        state[neuron_index] = next_state
        updates.append(
            {
                "step": step,
                "neuron_index": neuron_index,
                "state_before": state_before,
                "incoming": incoming,
                "local_field": local_field,
                "previous_state": previous_state,
                "next_state": next_state,
                "changed": next_state != previous_state,
                "state_after": state[:],
                "energy_before": hopfield_energy(state_before, weights),
                "energy_after": hopfield_energy(state, weights),
                "overlap_before": normalized_overlap(pattern, state_before),
                "overlap_after": normalized_overlap(pattern, state),
            }
        )

    final_distance = hamming_distance(pattern, state)
    return {
        "normalization": len(pattern),
        "weights": weights,
        "initial_energy": hopfield_energy(corrupted, weights),
        "initial_overlap": normalized_overlap(pattern, corrupted),
        "initial_hamming_distance": hamming_distance(pattern, corrupted),
        "updates": updates,
        "final_state": state,
        "final_energy": hopfield_energy(state, weights),
        "final_overlap": normalized_overlap(pattern, state),
        "final_hamming_distance": final_distance,
        "converged": final_distance == 0
        and all(row["energy_after"] <= row["energy_before"] + 1e-12 for row in updates),
    }


def _compare(actual: Any, expected: Any, tolerance: float, context: str) -> None:
    if isinstance(expected, bool):
        if actual is not expected:
            raise HopfieldAssociativeMemoryValidationError(
                f"{context}: expected {expected}, got {actual}"
            )
        return
    if isinstance(expected, (int, float)) and not isinstance(expected, bool):
        actual_number = _number(actual, context)
        if abs(actual_number - float(expected)) > tolerance:
            raise HopfieldAssociativeMemoryValidationError(
                f"{context}: expected {expected}, got {actual_number}"
            )
        return
    if isinstance(expected, str):
        if actual != expected:
            raise HopfieldAssociativeMemoryValidationError(
                f"{context}: expected {expected!r}, got {actual!r}"
            )
        return
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise HopfieldAssociativeMemoryValidationError(
                f"{context}: expected an array of length {len(expected)}"
            )
        for index, (actual_item, expected_item) in enumerate(zip(actual, expected)):
            _compare(actual_item, expected_item, tolerance, f"{context}[{index}]")
        return
    if isinstance(expected, dict):
        actual_object = _object(actual, set(expected), context)
        for key, expected_value in expected.items():
            _compare(actual_object[key], expected_value, tolerance, f"{context}.{key}")
        return
    raise HopfieldAssociativeMemoryValidationError(
        f"{context}: unsupported expected value {expected!r}"
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
            "concepts",
            "operation",
            "stored_pattern",
            "corrupted_state",
            "update_order",
            "expected",
        },
        "document",
    )
    if _integer(root["schema_version"], "schema_version") != 1:
        raise HopfieldAssociativeMemoryValidationError("schema_version must be 1")
    for key in ("id", "title", "question"):
        if not isinstance(root[key], str) or not root[key].strip():
            raise HopfieldAssociativeMemoryValidationError(
                f"{key}: expected a non-empty string"
            )
    tolerance = _number(root["absolute_tolerance"], "absolute_tolerance")
    if tolerance <= 0:
        raise HopfieldAssociativeMemoryValidationError(
            "absolute_tolerance must be positive"
        )
    concepts = root["concepts"]
    if (
        not isinstance(concepts, list)
        or not concepts
        or any(not isinstance(item, str) or not item for item in concepts)
        or len(set(concepts)) != len(concepts)
    ):
        raise HopfieldAssociativeMemoryValidationError(
            "concepts must contain unique non-empty strings"
        )

    operation = _object(
        root["operation"],
        {"kind", "storage", "activation", "update", "energy"},
        "operation",
    )
    required_operation = {
        "kind": "hopfield-associative-recall",
        "storage": "single-pattern-normalized-hebbian",
        "activation": "sign-preserve-zero",
        "update": "asynchronous-in-place",
        "energy": "symmetric-zero-diagonal-hopfield",
    }
    if operation != required_operation:
        raise HopfieldAssociativeMemoryValidationError(
            "operation does not match the NN20 V1 contract"
        )

    actual = execute_lab(root)
    expected = root["expected"]
    if not isinstance(expected, dict):
        raise HopfieldAssociativeMemoryValidationError("expected: expected an object")
    _compare(actual, expected, tolerance, "expected")

    weights = actual["weights"]
    for row in range(len(weights)):
        if abs(weights[row][row]) > tolerance:
            raise HopfieldAssociativeMemoryValidationError(
                "weight matrix diagonal must be zero"
            )
        for column in range(len(weights)):
            if abs(weights[row][column] - weights[column][row]) > tolerance:
                raise HopfieldAssociativeMemoryValidationError(
                    "weight matrix must be symmetric"
                )
    if any(
        row["energy_after"] > row["energy_before"] + tolerance
        for row in actual["updates"]
    ):
        raise HopfieldAssociativeMemoryValidationError(
            "asynchronous Hopfield energy must not increase"
        )
    if actual["final_overlap"] <= actual["initial_overlap"]:
        raise HopfieldAssociativeMemoryValidationError(
            "fixture must improve normalized overlap"
        )
    if not actual["converged"]:
        raise HopfieldAssociativeMemoryValidationError(
            "fixture must recover the saved fixed point"
        )
    return actual


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> int:
    schema_path = fixture_root / "schema.json"
    load_json(schema_path)
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise HopfieldAssociativeMemoryValidationError(
            f"no lab documents found under {fixture_root / 'labs'}"
        )
    seen_ids: set[str] = set()
    for path in lab_paths:
        document = load_json(path)
        lab_id = document.get("id")
        if lab_id in seen_ids:
            raise HopfieldAssociativeMemoryValidationError(
                f"duplicate lab id: {lab_id}"
            )
        seen_ids.add(str(lab_id))
        validate_document(document)
        print(f"validated {path}")
    return len(lab_paths)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixture-root",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
        help="Hopfield V1 fixture directory",
    )
    args = parser.parse_args()
    count = validate_corpus(args.fixture_root)
    print(f"validated {count} NN20 Hopfield associative-memory lab document(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
