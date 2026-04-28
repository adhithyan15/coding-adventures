"""Single-layer multi-input multi-output neural network primitives."""

from __future__ import annotations

from dataclasses import dataclass
from math import exp
from typing import Literal

ActivationName = Literal["linear", "sigmoid"]
Matrix = list[list[float]]

__version__ = "0.1.0"


@dataclass(frozen=True)
class TrainingStep:
    predictions: Matrix
    errors: Matrix
    weight_gradients: Matrix
    bias_gradients: list[float]
    next_weights: Matrix
    next_biases: list[float]
    loss: float


def _validate_matrix(name: str, matrix: Matrix) -> tuple[int, int]:
    if not matrix:
        raise ValueError(f"{name} must contain at least one row")
    width = len(matrix[0])
    if width == 0:
        raise ValueError(f"{name} must contain at least one column")
    for row in matrix:
        if len(row) != width:
            raise ValueError(f"{name} must be rectangular")
    return len(matrix), width


def _activate(value: float, activation: ActivationName) -> float:
    if activation == "linear":
        return value
    if activation == "sigmoid":
        if value >= 0:
            z = exp(-value)
            return 1.0 / (1.0 + z)
        z = exp(value)
        return z / (1.0 + z)
    raise ValueError(f"unsupported activation: {activation}")


def _derivative_from_output(output: float, activation: ActivationName) -> float:
    if activation == "linear":
        return 1.0
    if activation == "sigmoid":
        return output * (1.0 - output)
    raise ValueError(f"unsupported activation: {activation}")


def predict_with_parameters(
    inputs: Matrix,
    weights: Matrix,
    biases: list[float],
    activation: ActivationName = "linear",
) -> Matrix:
    sample_count, input_count = _validate_matrix("inputs", inputs)
    weight_rows, output_count = _validate_matrix("weights", weights)
    if input_count != weight_rows:
        raise ValueError("input column count must match weight row count")
    if len(biases) != output_count:
        raise ValueError("bias count must match output count")

    predictions: Matrix = []
    for sample_index in range(sample_count):
        row: list[float] = []
        for output_index in range(output_count):
            total = biases[output_index]
            for input_index in range(input_count):
                total += inputs[sample_index][input_index] * weights[input_index][output_index]
            row.append(_activate(total, activation))
        predictions.append(row)
    return predictions


def train_one_epoch_with_matrices(
    inputs: Matrix,
    targets: Matrix,
    weights: Matrix,
    biases: list[float],
    learning_rate: float,
    activation: ActivationName = "linear",
) -> TrainingStep:
    sample_count, input_count = _validate_matrix("inputs", inputs)
    target_rows, output_count = _validate_matrix("targets", targets)
    weight_rows, weight_cols = _validate_matrix("weights", weights)
    if target_rows != sample_count:
        raise ValueError("inputs and targets must have the same row count")
    if weight_rows != input_count or weight_cols != output_count:
        raise ValueError("weights must be shaped input_count x output_count")
    if len(biases) != output_count:
        raise ValueError("bias count must match output count")

    predictions = predict_with_parameters(inputs, weights, biases, activation)
    scale = 2.0 / float(sample_count * output_count)
    errors: Matrix = []
    deltas: Matrix = []
    loss_total = 0.0
    for row_index in range(sample_count):
        error_row: list[float] = []
        delta_row: list[float] = []
        for output_index in range(output_count):
            error = predictions[row_index][output_index] - targets[row_index][output_index]
            error_row.append(error)
            delta_row.append(scale * error * _derivative_from_output(predictions[row_index][output_index], activation))
            loss_total += error * error
        errors.append(error_row)
        deltas.append(delta_row)

    weight_gradients = [[0.0 for _ in range(output_count)] for _ in range(input_count)]
    bias_gradients = [0.0 for _ in range(output_count)]
    for input_index in range(input_count):
        for output_index in range(output_count):
            weight_gradients[input_index][output_index] = sum(
                inputs[row_index][input_index] * deltas[row_index][output_index]
                for row_index in range(sample_count)
            )
    for output_index in range(output_count):
        bias_gradients[output_index] = sum(deltas[row_index][output_index] for row_index in range(sample_count))

    next_weights = [
        [
            weights[input_index][output_index] - learning_rate * weight_gradients[input_index][output_index]
            for output_index in range(output_count)
        ]
        for input_index in range(input_count)
    ]
    next_biases = [
        biases[output_index] - learning_rate * bias_gradients[output_index]
        for output_index in range(output_count)
    ]

    return TrainingStep(
        predictions=predictions,
        errors=errors,
        weight_gradients=weight_gradients,
        bias_gradients=bias_gradients,
        next_weights=next_weights,
        next_biases=next_biases,
        loss=loss_total / float(sample_count * output_count),
    )


class SingleLayerNetwork:
    def __init__(self, weights: Matrix, biases: list[float], activation: ActivationName = "linear"):
        self.weights = weights
        self.biases = biases
        self.activation = activation

    @classmethod
    def with_shape(cls, input_count: int, output_count: int, activation: ActivationName = "linear") -> "SingleLayerNetwork":
        return cls([[0.0 for _ in range(output_count)] for _ in range(input_count)], [0.0 for _ in range(output_count)], activation)

    def predict(self, inputs: Matrix) -> Matrix:
        return predict_with_parameters(inputs, self.weights, self.biases, self.activation)

    def fit(self, inputs: Matrix, targets: Matrix, learning_rate: float = 0.05, epochs: int = 100) -> list[TrainingStep]:
        history: list[TrainingStep] = []
        for _ in range(epochs):
            step = train_one_epoch_with_matrices(inputs, targets, self.weights, self.biases, learning_rate, self.activation)
            self.weights = step.next_weights
            self.biases = step.next_biases
            history.append(step)
        return history


def fit_single_layer_network(
    inputs: Matrix,
    targets: Matrix,
    learning_rate: float = 0.05,
    epochs: int = 100,
    activation: ActivationName = "linear",
) -> SingleLayerNetwork:
    _, input_count = _validate_matrix("inputs", inputs)
    _, output_count = _validate_matrix("targets", targets)
    model = SingleLayerNetwork.with_shape(input_count, output_count, activation)
    model.fit(inputs, targets, learning_rate, epochs)
    return model
