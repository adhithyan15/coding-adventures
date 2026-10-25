"""NN37: a reusable dense training framework above the matrix foundation.

A layer is a little machine: multiply the previous activations by its weights,
add one bias per output, then apply an activation. Backpropagation walks these
machines in reverse and tells every weight how it contributed to the loss.
The same equations and checkpoint format are implemented independently in TS.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, isfinite, sqrt, tanh
from typing import Literal

from matrix import Matrix

Activation = Literal["linear", "sigmoid", "tanh", "relu"]
Grid = list[list[float]]
_ACTIVATIONS = ("linear", "sigmoid", "tanh", "relu")
_MAX_STEP = 2**53 - 1
__version__ = "0.1.0"


def _number(value: object) -> float:
    if type(value) not in (int, float):
        raise ValueError("expected a finite number")
    try:
        result = float(value)
    except OverflowError as error:
        raise ValueError("expected a finite number") from error
    if not isfinite(result):
        raise ValueError("expected a finite number")
    return result


def _count(value: object, minimum: int = 0, maximum: int = _MAX_STEP) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError("count must be an integer in range")
    return value


def _vector(value: object) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError("expected a nonempty numeric array")
    return [_number(item) for item in value]


def _grid(value: object) -> Grid:
    if not isinstance(value, list) or not value:
        raise ValueError("expected a nonempty matrix")
    rows = [_vector(row) for row in value]
    if any(len(row) != len(rows[0]) for row in rows):
        raise ValueError("matrix must be rectangular")
    return rows


def _activate(value: float, activation: Activation) -> float:
    if activation == "linear":
        return value
    if activation == "relu":
        return max(0.0, value)
    if activation == "tanh":
        return tanh(value)
    # Both branches use exp(nonpositive), so sigmoid never overflows.
    factor = exp(-abs(value))
    return 1 / (1 + factor) if value >= 0 else factor / (1 + factor)


def _derivative(raw: float, activated: float, activation: Activation) -> float:
    if activation == "linear":
        return 1.0
    if activation == "relu":
        return float(raw > 0)
    if activation == "tanh":
        return 1 - activated * activated
    return activated * (1 - activated)


@dataclass(frozen=True)
class DenseLayer:
    """Weights use [input][output] order, matching A @ W in the matrix package."""

    weights: Grid
    biases: list[float]
    activation: Activation = "linear"


@dataclass(frozen=True)
class GradientTrace:
    """A snapshot at one parameter state, before any optimizer update."""

    loss: float
    predictions: Grid
    weight_gradients: list[Grid]
    bias_gradients: list[list[float]]


class DenseNetwork:
    """Sequential dense layers with analytical gradients and full-batch SGD.

    Public inputs and outputs are copied. A failed update cannot leave half
    the layers changed: the complete candidate is evaluated before adoption.
    """

    def __init__(self, layers: list[DenseLayer], *, step: int = 0):
        if not isinstance(layers, list) or not layers:
            raise ValueError("network needs at least one layer")
        self._step = _count(step)
        self._layers: list[DenseLayer] = []
        for layer in layers:
            if not isinstance(layer, DenseLayer) or layer.activation not in _ACTIVATIONS:
                raise ValueError("unsupported dense layer")
            weights, biases = _grid(layer.weights), _vector(layer.biases)
            if len(weights[0]) != len(biases):
                raise ValueError("weight columns must match bias count")
            if self._layers and len(weights) != len(self._layers[-1].biases):
                raise ValueError("consecutive layer widths do not match")
            self._layers.append(DenseLayer(weights, biases, layer.activation))

    @property
    def step(self) -> int:
        return self._step

    @classmethod
    def seeded(cls, widths: list[int], activations: list[Activation], seed: int = 7):
        """LCG initialization shared with other languages, never a warm start."""
        if not isinstance(widths, list) or len(widths) < 2:
            raise ValueError("widths must include input and output")
        widths = [_count(width, 1) for width in widths]
        if not isinstance(activations, list) or len(activations) != len(widths) - 1:
            raise ValueError("one activation is required per layer")
        if any(activation not in _ACTIVATIONS for activation in activations):
            raise ValueError("unsupported activation")
        state = _count(seed, maximum=2**32 - 1)
        layers = []
        for fan_in, fan_out, activation in zip(widths[:-1], widths[1:], activations, strict=True):
            weights = []
            for _ in range(fan_in):
                row = []
                for _ in range(fan_out):
                    state = (1664525 * state + 1013904223) % 2**32
                    row.append((2 * state / 2**32 - 1) / sqrt(fan_in))
                weights.append(row)
            layers.append(DenseLayer(weights, [0.0] * fan_out, activation))
        return cls(layers)

    def _forward(self, inputs: Grid) -> tuple[list[Grid], list[Grid]]:
        current = _grid(inputs)
        if len(current[0]) != len(self._layers[0].weights):
            raise ValueError("input width does not match model")
        saved, raw_layers = [current], []
        for layer in self._layers:
            products = Matrix(current).dot(Matrix(layer.weights)).data
            raw = _grid([[v + layer.biases[j] for j, v in enumerate(row)] for row in products])
            current = _grid([[_activate(v, layer.activation) for v in row] for row in raw])
            raw_layers.append(raw)
            saved.append(current)
        return saved, raw_layers

    def predict(self, inputs: Grid) -> Grid:
        return self._forward(inputs)[0][-1]

    def gradients(self, inputs: Grid, targets: Grid) -> GradientTrace:
        """Reverse-mode differentiation for every layer, including multiple outputs.

        MSE divides by samples * outputs, rather than only samples. This small
        distinction prevents learning-rate drift when output width changes.
        """
        saved, raw_layers = self._forward(inputs)
        target = _grid(targets)
        predictions = saved[-1]
        if len(target) != len(predictions) or len(target[0]) != len(predictions[0]):
            raise ValueError("target shape does not match predictions")
        errors = _grid(
            [
                [p - t for p, t in zip(row, wanted, strict=True)]
                for row, wanted in zip(predictions, target, strict=True)
            ]
        )
        count = len(errors) * len(errors[0])
        loss = _number(sum(e * e for row in errors for e in row) / count)
        upstream = _grid([[2 * e / count for e in row] for row in errors])
        weight_gradients, bias_gradients = [], []
        for index in range(len(self._layers) - 1, -1, -1):
            layer = self._layers[index]
            delta = _grid(
                [
                    [
                        g * _derivative(z, a, layer.activation)
                        for g, z, a in zip(gs, zs, acts, strict=True)
                    ]
                    for gs, zs, acts in zip(
                        upstream, raw_layers[index], saved[index + 1], strict=True
                    )
                ]
            )
            dw = _grid(Matrix(saved[index]).transpose().dot(Matrix(delta)).data)
            db = _vector([sum(column) for column in zip(*delta, strict=True)])
            weight_gradients.insert(0, dw)
            bias_gradients.insert(0, db)
            if index:
                upstream = _grid(Matrix(delta).dot(Matrix(layer.weights).transpose()).data)
        return GradientTrace(loss, predictions, weight_gradients, bias_gradients)

    def train_step(self, inputs: Grid, targets: Grid, learning_rate: float) -> GradientTrace:
        rate = _number(learning_rate)
        if rate <= 0:
            raise ValueError("learning rate must be positive")
        next_step = _count(self._step + 1)
        trace = self.gradients(inputs, targets)
        layers = []
        for layer, dw, db in zip(
            self._layers, trace.weight_gradients, trace.bias_gradients, strict=True
        ):
            layers.append(
                DenseLayer(
                    [
                        [v - rate * g for v, g in zip(row, gradients, strict=True)]
                        for row, gradients in zip(layer.weights, dw, strict=True)
                    ],
                    [v - rate * g for v, g in zip(layer.biases, db, strict=True)],
                    layer.activation,
                )
            )
        candidate = DenseNetwork(layers, step=next_step)
        candidate.gradients(inputs, targets)  # Refuse an overflowing candidate before mutation.
        self._layers, self._step = candidate._layers, candidate._step
        return trace

    def fit(self, inputs: Grid, targets: Grid, epochs: int, learning_rate: float) -> list[float]:
        epochs = _count(epochs)
        rate = _number(learning_rate)
        if rate <= 0:
            raise ValueError("learning rate must be positive")
        losses = [self.gradients(inputs, targets).loss]
        for _ in range(epochs):
            self.train_step(inputs, targets, rate)
            losses.append(self.gradients(inputs, targets).loss)
        return losses

    def to_checkpoint(self) -> dict:
        return {
            "format": "coding-adventures-dense",
            "version": 1,
            "step": self.step,
            "layers": [
                {
                    "activation": layer.activation,
                    "weights": [row[:] for row in layer.weights],
                    "biases": layer.biases[:],
                }
                for layer in self._layers
            ],
        }

    @classmethod
    def from_checkpoint(cls, document: object):
        """Load data only; never resolve a class or execute checkpoint content."""
        if type(document) is not dict or set(document) != {"format", "version", "step", "layers"}:
            raise ValueError("invalid checkpoint fields")
        if document["format"] != "coding-adventures-dense" or type(document["version"]) is not int:
            raise ValueError("unsupported checkpoint format")
        if document["version"] != 1 or not isinstance(document["layers"], list):
            raise ValueError("unsupported checkpoint version or layers")
        layers = []
        for layer in document["layers"]:
            if type(layer) is not dict or set(layer) != {"activation", "weights", "biases"}:
                raise ValueError("invalid checkpoint layer fields")
            layers.append(DenseLayer(layer["weights"], layer["biases"], layer["activation"]))
        return cls(layers, step=document["step"])
