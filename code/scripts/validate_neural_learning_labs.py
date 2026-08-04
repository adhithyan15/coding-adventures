#!/usr/bin/env python3
"""Validate and execute the deterministic NN03 neural-learning corpus."""

from __future__ import annotations

import argparse
import json
import math
from copy import deepcopy
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "neural-learning-v1"
ACTIVATIONS = {"identity", "sigmoid", "tanh", "relu"}


class LabValidationError(ValueError):
    """Raised when a learning lab is structurally or numerically invalid."""


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise LabValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=_reject_duplicate_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                LabValidationError(f"non-finite JSON number: {value}")
            ),
        )
    except (OSError, json.JSONDecodeError) as error:
        raise LabValidationError(f"{path}: {error}") from error
    if not isinstance(document, dict):
        raise LabValidationError(f"{path}: top-level JSON value must be an object")
    return document


def _require_keys(value: dict[str, Any], required: set[str], context: str) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        raise LabValidationError(f"{context}: missing keys {sorted(missing)}")
    if extra:
        raise LabValidationError(f"{context}: unexpected keys {sorted(extra)}")


def _require_number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise LabValidationError(f"{context}: expected a number")
    number = float(value)
    if not math.isfinite(number):
        raise LabValidationError(f"{context}: expected a finite number")
    return number


def _number_vector(value: Any, context: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise LabValidationError(f"{context}: expected a non-empty number vector")
    return [
        _require_number(item, f"{context}[{index}]") for index, item in enumerate(value)
    ]


def _number_matrix(value: Any, context: str) -> list[list[float]]:
    if not isinstance(value, list) or not value:
        raise LabValidationError(f"{context}: expected a non-empty number matrix")
    rows = [
        _number_vector(row, f"{context}[{index}]") for index, row in enumerate(value)
    ]
    width = len(rows[0])
    if any(len(row) != width for row in rows):
        raise LabValidationError(f"{context}: rows must have equal widths")
    return rows


def _activation(name: str, value: float) -> float:
    if name == "identity":
        return value
    if name == "sigmoid":
        if value >= 0:
            return 1.0 / (1.0 + math.exp(-value))
        exp_value = math.exp(value)
        return exp_value / (1.0 + exp_value)
    if name == "tanh":
        return math.tanh(value)
    if name == "relu":
        return max(0.0, value)
    raise LabValidationError(f"unsupported activation: {name}")


def _activation_derivative(name: str, raw: float, output: float) -> float:
    if name == "identity":
        return 1.0
    if name == "sigmoid":
        return output * (1.0 - output)
    if name == "tanh":
        return 1.0 - output * output
    if name == "relu":
        return 1.0 if raw > 0 else 0.0
    raise LabValidationError(f"unsupported activation: {name}")


def _matmul(left: list[list[float]], right: list[list[float]]) -> list[list[float]]:
    if not left or not right or len(left[0]) != len(right):
        raise LabValidationError("matrix multiplication shape mismatch")
    return [
        [
            sum(row[k] * right[k][column] for k in range(len(right)))
            for column in range(len(right[0]))
        ]
        for row in left
    ]


def _transpose(matrix: list[list[float]]) -> list[list[float]]:
    return [list(column) for column in zip(*matrix, strict=True)]


def validate_structure(lab: dict[str, Any], source: str = "lab") -> None:
    _require_keys(
        lab,
        {
            "schema_version",
            "id",
            "title",
            "stage",
            "question",
            "concepts",
            "model",
            "dataset",
            "training",
            "expected",
        },
        source,
    )
    if lab["schema_version"] != 1:
        raise LabValidationError(f"{source}: schema_version must be 1")
    if not isinstance(lab["id"], str) or not lab["id"]:
        raise LabValidationError(f"{source}: id must be a non-empty string")
    if lab["stage"] not in {"forward", "single-neuron", "hidden-layer"}:
        raise LabValidationError(f"{source}: invalid stage")
    if not isinstance(lab["concepts"], list) or not lab["concepts"]:
        raise LabValidationError(f"{source}: concepts must be a non-empty list")

    model = lab["model"]
    if not isinstance(model, dict):
        raise LabValidationError(f"{source}.model: expected an object")
    _require_keys(model, {"kind", "input_count", "layers"}, f"{source}.model")
    input_count = model["input_count"]
    if (
        not isinstance(input_count, int)
        or isinstance(input_count, bool)
        or input_count < 1
    ):
        raise LabValidationError(
            f"{source}.model.input_count: expected a positive integer"
        )
    if model["kind"] not in {"single-neuron", "dense-network"}:
        raise LabValidationError(f"{source}.model.kind: invalid model kind")
    if not isinstance(model["layers"], list) or not model["layers"]:
        raise LabValidationError(f"{source}.model.layers: expected a non-empty list")

    previous_width = input_count
    layer_names: set[str] = set()
    for index, layer in enumerate(model["layers"]):
        context = f"{source}.model.layers[{index}]"
        if not isinstance(layer, dict):
            raise LabValidationError(f"{context}: expected an object")
        _require_keys(layer, {"name", "weights", "biases", "activation"}, context)
        if not isinstance(layer["name"], str) or not layer["name"]:
            raise LabValidationError(f"{context}.name: expected a non-empty string")
        if layer["name"] in layer_names:
            raise LabValidationError(
                f"{context}.name: duplicate layer name {layer['name']!r}"
            )
        layer_names.add(layer["name"])
        if layer["activation"] not in ACTIVATIONS:
            raise LabValidationError(f"{context}.activation: unsupported activation")
        weights = _number_matrix(layer["weights"], f"{context}.weights")
        biases = _number_vector(layer["biases"], f"{context}.biases")
        if len(weights) != previous_width:
            raise LabValidationError(
                f"{context}.weights: expected {previous_width} input rows, got {len(weights)}"
            )
        if len(weights[0]) != len(biases):
            raise LabValidationError(
                f"{context}: weight output width must equal bias width"
            )
        previous_width = len(biases)

    if model["kind"] == "single-neuron" and (
        len(model["layers"]) != 1 or previous_width != 1
    ):
        raise LabValidationError(
            f"{source}.model: single-neuron must have one layer and one output"
        )

    dataset = lab["dataset"]
    if not isinstance(dataset, dict):
        raise LabValidationError(f"{source}.dataset: expected an object")
    _require_keys(
        dataset, {"input_labels", "target_labels", "rows"}, f"{source}.dataset"
    )
    if len(dataset["input_labels"]) != input_count:
        raise LabValidationError(
            f"{source}.dataset.input_labels: width must match input_count"
        )
    if len(dataset["target_labels"]) != previous_width:
        raise LabValidationError(
            f"{source}.dataset.target_labels: width must match model output"
        )
    if not isinstance(dataset["rows"], list) or not dataset["rows"]:
        raise LabValidationError(f"{source}.dataset.rows: expected a non-empty list")
    labels: set[str] = set()
    for index, row in enumerate(dataset["rows"]):
        context = f"{source}.dataset.rows[{index}]"
        if not isinstance(row, dict):
            raise LabValidationError(f"{context}: expected an object")
        _require_keys(row, {"label", "input", "target"}, context)
        if not isinstance(row["label"], str) or not row["label"]:
            raise LabValidationError(f"{context}.label: expected a non-empty string")
        if row["label"] in labels:
            raise LabValidationError(
                f"{context}.label: duplicate row label {row['label']!r}"
            )
        labels.add(row["label"])
        if len(_number_vector(row["input"], f"{context}.input")) != input_count:
            raise LabValidationError(f"{context}.input: width must match input_count")
        if len(_number_vector(row["target"], f"{context}.target")) != previous_width:
            raise LabValidationError(f"{context}.target: width must match model output")

    training = lab["training"]
    expected = lab["expected"]
    if not isinstance(expected, dict):
        raise LabValidationError(f"{source}.expected: expected an object")
    _require_keys(
        expected, {"absolute_tolerance", "forward", "first_step"}, f"{source}.expected"
    )
    tolerance = _require_number(
        expected["absolute_tolerance"], f"{source}.expected.absolute_tolerance"
    )
    if tolerance <= 0:
        raise LabValidationError(
            f"{source}.expected.absolute_tolerance: must be positive"
        )
    if not isinstance(expected["forward"], list) or len(expected["forward"]) != len(
        labels
    ):
        raise LabValidationError(
            f"{source}.expected.forward: must cover every dataset row"
        )

    if training is None:
        if expected["first_step"] is not None:
            raise LabValidationError(
                f"{source}.expected.first_step: must be null without training"
            )
    else:
        if not isinstance(training, dict):
            raise LabValidationError(f"{source}.training: expected an object or null")
        _require_keys(training, {"loss", "optimizer", "batch"}, f"{source}.training")
        if training["loss"] != "mean-squared-error" or training["batch"] != "full":
            raise LabValidationError(
                f"{source}.training: V1 requires full-batch mean squared error"
            )
        optimizer = training["optimizer"]
        if not isinstance(optimizer, dict):
            raise LabValidationError(f"{source}.training.optimizer: expected an object")
        _require_keys(
            optimizer, {"kind", "learning_rate"}, f"{source}.training.optimizer"
        )
        if (
            optimizer["kind"] != "sgd"
            or _require_number(optimizer["learning_rate"], "learning_rate") <= 0
        ):
            raise LabValidationError(
                f"{source}.training.optimizer: V1 requires positive-rate SGD"
            )
        if expected["first_step"] is None:
            raise LabValidationError(
                f"{source}.expected.first_step: required when training is enabled"
            )


def forward(
    lab: dict[str, Any], layers: list[dict[str, Any]] | None = None
) -> dict[str, Any]:
    active_layers = deepcopy(layers if layers is not None else lab["model"]["layers"])
    inputs = [
        [float(value) for value in row["input"]] for row in lab["dataset"]["rows"]
    ]
    activations: list[list[list[float]]] = [inputs]
    raw_by_layer: list[list[list[float]]] = []
    current = inputs
    for layer in active_layers:
        weights = [[float(value) for value in row] for row in layer["weights"]]
        biases = [float(value) for value in layer["biases"]]
        raw = _matmul(current, weights)
        raw = [
            [value + biases[column] for column, value in enumerate(row)] for row in raw
        ]
        current = [
            [_activation(layer["activation"], value) for value in row] for row in raw
        ]
        raw_by_layer.append(raw)
        activations.append(current)
    return {"raw": raw_by_layer, "activations": activations, "predictions": current}


def train_first_step(lab: dict[str, Any]) -> dict[str, Any]:
    if lab["training"] is None:
        raise LabValidationError("cannot train a lab whose training field is null")
    layers = deepcopy(lab["model"]["layers"])
    pass_result = forward(lab, layers)
    predictions = pass_result["predictions"]
    targets = [
        [float(value) for value in row["target"]] for row in lab["dataset"]["rows"]
    ]
    value_count = len(targets) * len(targets[0])
    errors = [
        [
            prediction - target
            for prediction, target in zip(prediction_row, target_row, strict=True)
        ]
        for prediction_row, target_row in zip(predictions, targets, strict=True)
    ]
    loss_before = sum(value * value for row in errors for value in row) / value_count
    deltas: list[list[list[float]]] = [[] for _ in layers]
    last = len(layers) - 1
    deltas[last] = [
        [
            (2.0 / value_count)
            * error
            * _activation_derivative(
                layers[last]["activation"],
                pass_result["raw"][last][row_index][column],
                predictions[row_index][column],
            )
            for column, error in enumerate(error_row)
        ]
        for row_index, error_row in enumerate(errors)
    ]

    for layer_index in range(last - 1, -1, -1):
        downstream = _matmul(
            deltas[layer_index + 1], _transpose(layers[layer_index + 1]["weights"])
        )
        deltas[layer_index] = [
            [
                downstream[row_index][column]
                * _activation_derivative(
                    layers[layer_index]["activation"],
                    pass_result["raw"][layer_index][row_index][column],
                    pass_result["activations"][layer_index + 1][row_index][column],
                )
                for column in range(len(downstream[row_index]))
            ]
            for row_index in range(len(downstream))
        ]

    learning_rate = float(lab["training"]["optimizer"]["learning_rate"])
    gradients: list[dict[str, Any]] = []
    next_layers: list[dict[str, Any]] = []
    for layer_index, layer in enumerate(layers):
        previous = pass_result["activations"][layer_index]
        weight_gradient = _matmul(_transpose(previous), deltas[layer_index])
        bias_gradient = [
            sum(row[column] for row in deltas[layer_index])
            for column in range(len(deltas[layer_index][0]))
        ]
        gradients.append(
            {"name": layer["name"], "weights": weight_gradient, "biases": bias_gradient}
        )
        next_layers.append(
            {
                "name": layer["name"],
                "weights": [
                    [
                        value - learning_rate * weight_gradient[row][column]
                        for column, value in enumerate(values)
                    ]
                    for row, values in enumerate(layer["weights"])
                ],
                "biases": [
                    value - learning_rate * bias_gradient[index]
                    for index, value in enumerate(layer["biases"])
                ],
                "activation": layer["activation"],
            }
        )

    next_predictions = forward(lab, next_layers)["predictions"]
    next_errors = [
        [
            prediction - target
            for prediction, target in zip(prediction_row, target_row, strict=True)
        ]
        for prediction_row, target_row in zip(next_predictions, targets, strict=True)
    ]
    loss_after = (
        sum(value * value for row in next_errors for value in row) / value_count
    )
    return {
        "loss_before": loss_before,
        "gradients": gradients,
        "parameters_after": next_layers,
        "loss_after": loss_after,
    }


def _compare_numbers(
    actual: Any, expected: Any, tolerance: float, context: str
) -> None:
    if isinstance(expected, (int, float)) and not isinstance(expected, bool):
        actual_number = _require_number(actual, context)
        if abs(actual_number - float(expected)) > tolerance:
            raise LabValidationError(
                f"{context}: expected {expected!r}, got {actual_number!r} (tolerance {tolerance})"
            )
        return
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise LabValidationError(f"{context}: list shape mismatch")
        for index, (actual_item, expected_item) in enumerate(
            zip(actual, expected, strict=True)
        ):
            _compare_numbers(
                actual_item, expected_item, tolerance, f"{context}[{index}]"
            )
        return
    if isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            raise LabValidationError(f"{context}: object keys do not match")
        for key in expected:
            _compare_numbers(actual[key], expected[key], tolerance, f"{context}.{key}")
        return
    if actual != expected:
        raise LabValidationError(f"{context}: expected {expected!r}, got {actual!r}")


def validate_lab(lab: dict[str, Any], source: str = "lab") -> None:
    validate_structure(lab, source)
    tolerance = float(lab["expected"]["absolute_tolerance"])
    result = forward(lab)
    rows = lab["dataset"]["rows"]
    expected_by_label = {
        entry["row"]: entry["prediction"] for entry in lab["expected"]["forward"]
    }
    if set(expected_by_label) != {row["label"] for row in rows}:
        raise LabValidationError(
            f"{source}.expected.forward: row labels do not match dataset"
        )
    for index, row in enumerate(rows):
        _compare_numbers(
            result["predictions"][index],
            expected_by_label[row["label"]],
            tolerance,
            f"{source}.expected.forward[{row['label']}]",
        )
    if lab["training"] is not None:
        actual_step = train_first_step(lab)
        _compare_numbers(
            actual_step,
            lab["expected"]["first_step"],
            tolerance,
            f"{source}.expected.first_step",
        )


def validate_corpus(fixture_root: Path = DEFAULT_FIXTURE_ROOT) -> list[Path]:
    schema = load_json(fixture_root / "schema.json")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        raise LabValidationError("schema.json: expected JSON Schema Draft 2020-12")
    lab_paths = sorted((fixture_root / "labs").glob("*.json"))
    if not lab_paths:
        raise LabValidationError(f"{fixture_root}: no lab fixtures found")
    ids: set[str] = set()
    for path in lab_paths:
        lab = load_json(path)
        validate_lab(lab, str(path))
        if lab["id"] in ids:
            raise LabValidationError(f"{path}: duplicate lab id {lab['id']!r}")
        ids.add(lab["id"])
    return lab_paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    args = parser.parse_args()
    try:
        paths = validate_corpus(args.fixture_root)
    except LabValidationError as error:
        parser.exit(1, f"neural learning corpus invalid: {error}\n")
    print(f"validated {len(paths)} neural learning labs from {args.fixture_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
