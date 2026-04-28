"""Two-layer neural network primitives for hidden-layer learning examples."""

from __future__ import annotations

from dataclasses import dataclass
from math import exp
from typing import Literal

ActivationName = Literal["linear", "sigmoid"]
Matrix = list[list[float]]

__version__ = "0.1.0"


@dataclass(frozen=True)
class TwoLayerParameters:
    input_to_hidden_weights: Matrix
    hidden_biases: list[float]
    hidden_to_output_weights: Matrix
    output_biases: list[float]


@dataclass(frozen=True)
class ForwardPass:
    hidden_raw: Matrix
    hidden_activations: Matrix
    output_raw: Matrix
    predictions: Matrix


@dataclass(frozen=True)
class TrainingStep:
    hidden_activations: Matrix
    predictions: Matrix
    errors: Matrix
    output_deltas: Matrix
    hidden_deltas: Matrix
    hidden_to_output_weight_gradients: Matrix
    output_bias_gradients: list[float]
    input_to_hidden_weight_gradients: Matrix
    hidden_bias_gradients: list[float]
    next_parameters: TwoLayerParameters
    loss: float


@dataclass(frozen=True)
class TrainingSnapshot:
    epoch: int
    loss: float
    parameters: TwoLayerParameters
    predictions: Matrix
    hidden_activations: Matrix


def _validate_matrix(name: str, matrix: Matrix) -> tuple[int, int]:
    if not matrix:
        raise ValueError(f"{name} must contain at least one row")
    width = len(matrix[0])
    if width == 0:
        raise ValueError(f"{name} must contain at least one column")
    if any(len(row) != width for row in matrix):
        raise ValueError(f"{name} must be rectangular")
    return len(matrix), width


def _sigmoid(value: float) -> float:
    if value >= 0:
        z = exp(-value)
        return 1.0 / (1.0 + z)
    z = exp(value)
    return z / (1.0 + z)


def _activate(value: float, activation: ActivationName) -> float:
    if activation == "linear":
        return value
    if activation == "sigmoid":
        return _sigmoid(value)
    raise ValueError(f"unsupported activation: {activation}")


def _derivative(raw: float, activated: float, activation: ActivationName) -> float:
    if activation == "linear":
        return 1.0
    if activation == "sigmoid":
        return activated * (1.0 - activated)
    raise ValueError(f"unsupported activation: {activation}")


def _dot(left: Matrix, right: Matrix) -> Matrix:
    rows, width = _validate_matrix("left", left)
    right_rows, cols = _validate_matrix("right", right)
    if width != right_rows:
        raise ValueError("matrix shapes do not align")
    return [
        [
            sum(left[row][k] * right[k][col] for k in range(width))
            for col in range(cols)
        ]
        for row in range(rows)
    ]


def _transpose(matrix: Matrix) -> Matrix:
    rows, cols = _validate_matrix("matrix", matrix)
    return [[matrix[row][col] for row in range(rows)] for col in range(cols)]


def _add_biases(matrix: Matrix, biases: list[float]) -> Matrix:
    return [[value + biases[col] for col, value in enumerate(row)] for row in matrix]


def _apply_activation(matrix: Matrix, activation: ActivationName) -> Matrix:
    return [[_activate(value, activation) for value in row] for row in matrix]


def _column_sums(matrix: Matrix) -> list[float]:
    _, cols = _validate_matrix("matrix", matrix)
    return [sum(row[col] for row in matrix) for col in range(cols)]


def _mean_squared_error(errors: Matrix) -> float:
    values = [value for row in errors for value in row]
    return sum(value * value for value in values) / float(len(values))


def _subtract_scaled(matrix: Matrix, gradients: Matrix, learning_rate: float) -> Matrix:
    return [
        [value - learning_rate * gradients[row][col] for col, value in enumerate(matrix_row)]
        for row, matrix_row in enumerate(matrix)
    ]


def create_xor_warm_start_parameters() -> TwoLayerParameters:
    return TwoLayerParameters(
        input_to_hidden_weights=[[4.0, -4.0], [4.0, -4.0]],
        hidden_biases=[-2.0, 6.0],
        hidden_to_output_weights=[[4.0], [4.0]],
        output_biases=[-6.0],
    )


def forward_two_layer(
    inputs: Matrix,
    parameters: TwoLayerParameters,
    hidden_activation: ActivationName = "sigmoid",
    output_activation: ActivationName = "sigmoid",
) -> ForwardPass:
    _, input_count = _validate_matrix("inputs", inputs)
    weight_rows, hidden_count = _validate_matrix("input_to_hidden_weights", parameters.input_to_hidden_weights)
    hidden_rows, output_count = _validate_matrix("hidden_to_output_weights", parameters.hidden_to_output_weights)
    if input_count != weight_rows:
        raise ValueError("input width must match input-to-hidden weight row count")
    if len(parameters.hidden_biases) != hidden_count:
        raise ValueError("hidden bias count must match hidden width")
    if hidden_count != hidden_rows:
        raise ValueError("hidden width must match hidden-to-output weight row count")
    if len(parameters.output_biases) != output_count:
        raise ValueError("output bias count must match output width")

    hidden_raw = _add_biases(_dot(inputs, parameters.input_to_hidden_weights), parameters.hidden_biases)
    hidden_activations = _apply_activation(hidden_raw, hidden_activation)
    output_raw = _add_biases(_dot(hidden_activations, parameters.hidden_to_output_weights), parameters.output_biases)
    predictions = _apply_activation(output_raw, output_activation)
    return ForwardPass(hidden_raw, hidden_activations, output_raw, predictions)


def train_one_epoch_two_layer(
    inputs: Matrix,
    targets: Matrix,
    parameters: TwoLayerParameters,
    learning_rate: float,
    hidden_activation: ActivationName = "sigmoid",
    output_activation: ActivationName = "sigmoid",
) -> TrainingStep:
    sample_count, _ = _validate_matrix("inputs", inputs)
    target_rows, output_count = _validate_matrix("targets", targets)
    if target_rows != sample_count:
        raise ValueError("inputs and targets must have the same row count")
    forward = forward_two_layer(inputs, parameters, hidden_activation, output_activation)
    scale = 2.0 / float(sample_count * output_count)
    errors = [
        [forward.predictions[row][col] - targets[row][col] for col in range(output_count)]
        for row in range(sample_count)
    ]
    output_deltas = [
        [
            scale * errors[row][col] * _derivative(forward.output_raw[row][col], forward.predictions[row][col], output_activation)
            for col in range(output_count)
        ]
        for row in range(sample_count)
    ]
    h2o_gradients = _dot(_transpose(forward.hidden_activations), output_deltas)
    output_bias_gradients = _column_sums(output_deltas)
    hidden_errors = _dot(output_deltas, _transpose(parameters.hidden_to_output_weights))
    hidden_width = len(parameters.hidden_biases)
    hidden_deltas = [
        [
            hidden_errors[row][hidden] * _derivative(forward.hidden_raw[row][hidden], forward.hidden_activations[row][hidden], hidden_activation)
            for hidden in range(hidden_width)
        ]
        for row in range(sample_count)
    ]
    i2h_gradients = _dot(_transpose(inputs), hidden_deltas)
    hidden_bias_gradients = _column_sums(hidden_deltas)
    next_parameters = TwoLayerParameters(
        input_to_hidden_weights=_subtract_scaled(parameters.input_to_hidden_weights, i2h_gradients, learning_rate),
        hidden_biases=[
            bias - learning_rate * hidden_bias_gradients[index]
            for index, bias in enumerate(parameters.hidden_biases)
        ],
        hidden_to_output_weights=_subtract_scaled(parameters.hidden_to_output_weights, h2o_gradients, learning_rate),
        output_biases=[
            bias - learning_rate * output_bias_gradients[index]
            for index, bias in enumerate(parameters.output_biases)
        ],
    )
    return TrainingStep(
        hidden_activations=forward.hidden_activations,
        predictions=forward.predictions,
        errors=errors,
        output_deltas=output_deltas,
        hidden_deltas=hidden_deltas,
        hidden_to_output_weight_gradients=h2o_gradients,
        output_bias_gradients=output_bias_gradients,
        input_to_hidden_weight_gradients=i2h_gradients,
        hidden_bias_gradients=hidden_bias_gradients,
        next_parameters=next_parameters,
        loss=_mean_squared_error(errors),
    )


class TwoLayerNetwork:
    def __init__(
        self,
        parameters: TwoLayerParameters,
        learning_rate: float = 0.5,
        hidden_activation: ActivationName = "sigmoid",
        output_activation: ActivationName = "sigmoid",
    ):
        self.parameters = parameters
        self.learning_rate = learning_rate
        self.hidden_activation = hidden_activation
        self.output_activation = output_activation

    def fit(self, inputs: Matrix, targets: Matrix, epochs: int = 1000, log_every: int | None = None) -> list[TrainingSnapshot]:
        every = log_every if log_every is not None else epochs
        history: list[TrainingSnapshot] = []
        for epoch in range(epochs + 1):
            step = train_one_epoch_two_layer(
                inputs,
                targets,
                self.parameters,
                self.learning_rate,
                self.hidden_activation,
                self.output_activation,
            )
            self.parameters = step.next_parameters
            if epoch % every == 0 or epoch == epochs:
                history.append(TrainingSnapshot(epoch, step.loss, self.parameters, step.predictions, step.hidden_activations))
        return history

    def predict(self, inputs: Matrix) -> Matrix:
        return forward_two_layer(inputs, self.parameters, self.hidden_activation, self.output_activation).predictions

    def inspect(self, inputs: Matrix) -> ForwardPass:
        return forward_two_layer(inputs, self.parameters, self.hidden_activation, self.output_activation)
