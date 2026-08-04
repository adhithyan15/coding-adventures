#!/usr/bin/env python3
"""Validate and execute the deterministic NN04 optimization corpus."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "optimization-learning-v1"
)
STRATEGIES = {"stochastic", "mini-batch", "full-batch"}


class OptimizationLabValidationError(ValueError):
    """Raised when an optimization lab is structurally or numerically invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise OptimizationLabValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                OptimizationLabValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise OptimizationLabValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise OptimizationLabValidationError(
            f"{path}: top-level JSON value must be an object"
        )
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise OptimizationLabValidationError(
            f"{context}: missing keys {sorted(missing)}"
        )
    if extra:
        raise OptimizationLabValidationError(
            f"{context}: unexpected keys {sorted(extra)}"
        )


def _number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise OptimizationLabValidationError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise OptimizationLabValidationError(f"{context}: expected a finite number")
    return result


def _positive_number(value: Any, context: str) -> float:
    result = _number(value, context)
    if result <= 0:
        raise OptimizationLabValidationError(f"{context}: expected a positive number")
    return result


def _parameters(value: Any, context: str) -> dict[str, float]:
    if not isinstance(value, dict):
        raise OptimizationLabValidationError(f"{context}: expected an object")
    _require_keys(value, {"weight", "bias"}, context)
    return {
        "weight": _number(value["weight"], f"{context}.weight"),
        "bias": _number(value["bias"], f"{context}.bias"),
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
            "model",
            "dataset",
            "loss",
            "gradient_check",
            "optimizer_comparison",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise OptimizationLabValidationError(f"{source}.schema_version: expected 1")
    for field in ("id", "title", "question"):
        if not isinstance(lab[field], str) or not lab[field]:
            raise OptimizationLabValidationError(
                f"{source}.{field}: expected a non-empty string"
            )
    if not isinstance(lab["concepts"], list) or not lab["concepts"]:
        raise OptimizationLabValidationError(
            f"{source}.concepts: expected a non-empty list"
        )
    if len(set(lab["concepts"])) != len(lab["concepts"]):
        raise OptimizationLabValidationError(
            f"{source}.concepts: values must be unique"
        )

    model = lab["model"]
    if not isinstance(model, dict):
        raise OptimizationLabValidationError(f"{source}.model: expected an object")
    _require_keys(model, {"kind", "parameters"}, f"{source}.model")
    if model["kind"] != "linear-neuron":
        raise OptimizationLabValidationError(
            f"{source}.model.kind: V1 requires linear-neuron"
        )
    _parameters(model["parameters"], f"{source}.model.parameters")

    dataset = lab["dataset"]
    if not isinstance(dataset, dict):
        raise OptimizationLabValidationError(f"{source}.dataset: expected an object")
    _require_keys(dataset, {"rows"}, f"{source}.dataset")
    rows = dataset["rows"]
    if not isinstance(rows, list) or len(rows) < 2:
        raise OptimizationLabValidationError(
            f"{source}.dataset.rows: expected at least two rows"
        )
    labels: set[str] = set()
    for index, row in enumerate(rows):
        context = f"{source}.dataset.rows[{index}]"
        if not isinstance(row, dict):
            raise OptimizationLabValidationError(f"{context}: expected an object")
        _require_keys(row, {"label", "x", "target"}, context)
        if not isinstance(row["label"], str) or not row["label"]:
            raise OptimizationLabValidationError(
                f"{context}.label: expected a non-empty string"
            )
        if row["label"] in labels:
            raise OptimizationLabValidationError(
                f"{context}.label: duplicate label {row['label']!r}"
            )
        labels.add(row["label"])
        _number(row["x"], f"{context}.x")
        _number(row["target"], f"{context}.target")

    loss = lab["loss"]
    if not isinstance(loss, dict):
        raise OptimizationLabValidationError(f"{source}.loss: expected an object")
    _require_keys(loss, {"kind", "reduction"}, f"{source}.loss")
    if loss != {"kind": "mean-squared-error", "reduction": "mean"}:
        raise OptimizationLabValidationError(
            f"{source}.loss: V1 requires mean mean-squared-error"
        )

    gradient_check = lab["gradient_check"]
    if not isinstance(gradient_check, dict):
        raise OptimizationLabValidationError(
            f"{source}.gradient_check: expected an object"
        )
    _require_keys(gradient_check, {"epsilon", "expected"}, f"{source}.gradient_check")
    _positive_number(gradient_check["epsilon"], f"{source}.gradient_check.epsilon")
    expected = gradient_check["expected"]
    if not isinstance(expected, dict):
        raise OptimizationLabValidationError(
            f"{source}.gradient_check.expected: expected an object"
        )
    _require_keys(
        expected,
        {"absolute_tolerance", "loss", "analytical", "numerical"},
        f"{source}.gradient_check.expected",
    )
    _positive_number(
        expected["absolute_tolerance"],
        f"{source}.gradient_check.expected.absolute_tolerance",
    )
    _number(expected["loss"], f"{source}.gradient_check.expected.loss")
    _parameters(expected["analytical"], f"{source}.gradient_check.expected.analytical")
    _parameters(expected["numerical"], f"{source}.gradient_check.expected.numerical")

    comparison = lab["optimizer_comparison"]
    if not isinstance(comparison, dict):
        raise OptimizationLabValidationError(
            f"{source}.optimizer_comparison: expected an object"
        )
    _require_keys(
        comparison,
        {"learning_rate", "steps", "absolute_tolerance", "strategies"},
        f"{source}.optimizer_comparison",
    )
    _positive_number(
        comparison["learning_rate"],
        f"{source}.optimizer_comparison.learning_rate",
    )
    if (
        not isinstance(comparison["steps"], int)
        or isinstance(comparison["steps"], bool)
        or comparison["steps"] < 1
    ):
        raise OptimizationLabValidationError(
            f"{source}.optimizer_comparison.steps: expected a positive integer"
        )
    _positive_number(
        comparison["absolute_tolerance"],
        f"{source}.optimizer_comparison.absolute_tolerance",
    )
    strategies = comparison["strategies"]
    if not isinstance(strategies, list) or len(strategies) != 3:
        raise OptimizationLabValidationError(
            f"{source}.optimizer_comparison.strategies: expected three strategies"
        )
    found: set[str] = set()
    expected_batch_sizes = {
        "stochastic": 1,
        "mini-batch": 2,
        "full-batch": len(rows),
    }
    for index, strategy in enumerate(strategies):
        context = f"{source}.optimizer_comparison.strategies[{index}]"
        if not isinstance(strategy, dict):
            raise OptimizationLabValidationError(f"{context}: expected an object")
        _require_keys(
            strategy,
            {"kind", "batch_size", "expected_parameters", "expected_loss"},
            context,
        )
        kind = strategy["kind"]
        if kind not in STRATEGIES or kind in found:
            raise OptimizationLabValidationError(
                f"{context}.kind: expected each V1 strategy exactly once"
            )
        found.add(kind)
        if strategy["batch_size"] != expected_batch_sizes[kind]:
            raise OptimizationLabValidationError(
                f"{context}.batch_size: inconsistent with {kind}"
            )
        _parameters(strategy["expected_parameters"], f"{context}.expected_parameters")
        _number(strategy["expected_loss"], f"{context}.expected_loss")


def _rows(lab: dict[str, Any]) -> list[dict[str, float]]:
    return [
        {"x": float(row["x"]), "target": float(row["target"])}
        for row in lab["dataset"]["rows"]
    ]


def mean_squared_error(
    rows: list[dict[str, float]], parameters: dict[str, float]
) -> float:
    return sum(
        (parameters["weight"] * row["x"] + parameters["bias"] - row["target"]) ** 2
        for row in rows
    ) / len(rows)


def analytical_gradient(
    rows: list[dict[str, float]], parameters: dict[str, float]
) -> dict[str, float]:
    scale = 2.0 / len(rows)
    weight = 0.0
    bias = 0.0
    for row in rows:
        error = parameters["weight"] * row["x"] + parameters["bias"] - row["target"]
        weight += scale * error * row["x"]
        bias += scale * error
    return {"weight": weight, "bias": bias}


def numerical_gradient(
    rows: list[dict[str, float]], parameters: dict[str, float], epsilon: float
) -> dict[str, float]:
    result: dict[str, float] = {}
    for field in ("weight", "bias"):
        plus = dict(parameters)
        minus = dict(parameters)
        plus[field] += epsilon
        minus[field] -= epsilon
        result[field] = (
            mean_squared_error(rows, plus) - mean_squared_error(rows, minus)
        ) / (2.0 * epsilon)
    return result


def batch_indices(kind: str, step: int, row_count: int) -> list[int]:
    if kind == "full-batch":
        return list(range(row_count))
    if kind == "stochastic":
        return [step % row_count]
    start = (step * 2) % row_count
    return [start, (start + 1) % row_count]


def run_strategy(lab: dict[str, Any], kind: str) -> dict[str, Any]:
    rows = _rows(lab)
    parameters = _parameters(lab["model"]["parameters"], "model.parameters")
    comparison = lab["optimizer_comparison"]
    learning_rate = float(comparison["learning_rate"])
    for step in range(comparison["steps"]):
        selected = [rows[index] for index in batch_indices(kind, step, len(rows))]
        gradient = analytical_gradient(selected, parameters)
        parameters = {
            "weight": parameters["weight"] - learning_rate * gradient["weight"],
            "bias": parameters["bias"] - learning_rate * gradient["bias"],
        }
    return {"parameters": parameters, "loss": mean_squared_error(rows, parameters)}


def _compare(actual: float, expected: Any, tolerance: float, context: str) -> None:
    expected_number = _number(expected, context)
    if abs(actual - expected_number) > tolerance:
        raise OptimizationLabValidationError(
            f"{context}: expected {expected_number!r}, got {actual!r} "
            f"(tolerance {tolerance})"
        )


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    rows = _rows(lab)
    parameters = _parameters(lab["model"]["parameters"], f"{source}.model.parameters")
    gradient_check = lab["gradient_check"]
    expected_gradient = gradient_check["expected"]
    gradient_tolerance = float(expected_gradient["absolute_tolerance"])
    _compare(
        mean_squared_error(rows, parameters),
        expected_gradient["loss"],
        gradient_tolerance,
        f"{source}.gradient_check.expected.loss",
    )
    for name, actual in (
        ("analytical", analytical_gradient(rows, parameters)),
        (
            "numerical",
            numerical_gradient(rows, parameters, float(gradient_check["epsilon"])),
        ),
    ):
        for field in ("weight", "bias"):
            _compare(
                actual[field],
                expected_gradient[name][field],
                gradient_tolerance,
                f"{source}.gradient_check.expected.{name}.{field}",
            )

    comparison = lab["optimizer_comparison"]
    optimizer_tolerance = float(comparison["absolute_tolerance"])
    for strategy in comparison["strategies"]:
        actual = run_strategy(lab, strategy["kind"])
        for field in ("weight", "bias"):
            _compare(
                actual["parameters"][field],
                strategy["expected_parameters"][field],
                optimizer_tolerance,
                f"{source}.optimizer_comparison.{strategy['kind']}.{field}",
            )
        _compare(
            actual["loss"],
            strategy["expected_loss"],
            optimizer_tolerance,
            f"{source}.optimizer_comparison.{strategy['kind']}.loss",
        )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise OptimizationLabValidationError(
            "schema.json: expected JSON Schema Draft 2020-12"
        )
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise OptimizationLabValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise OptimizationLabValidationError(
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
    except OptimizationLabValidationError as error:
        parser.exit(1, f"optimization learning corpus invalid: {error}\n")
    print(f"validated {len(paths)} optimization learning labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
