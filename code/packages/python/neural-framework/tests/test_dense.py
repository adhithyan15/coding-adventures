"""Independent numerical and ownership evidence for the NN37 framework contract."""

import copy
import json
from pathlib import Path

import pytest

from neural_framework import DenseLayer, DenseNetwork

FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[4] / "specs/fixtures/neural-framework-v1/dense.json"
    ).read_text()
)


def close(actual, expected, tolerance=1e-12):
    if isinstance(expected, dict):
        assert actual.keys() == expected.keys()
        for key in expected:
            close(actual[key], expected[key], tolerance)
    elif isinstance(expected, list):
        assert len(actual) == len(expected)
        for left, right in zip(actual, expected, strict=True):
            close(left, right, tolerance)
    elif isinstance(expected, (int, float)):
        assert actual == pytest.approx(expected, abs=tolerance, rel=0)
    else:
        assert actual == expected


def test_paper_oracle_every_gradient_and_update():
    case = FIXTURE["paper"]
    assert FIXTURE["absolute_tolerance"] == 1e-12
    model = DenseNetwork.from_checkpoint(case["checkpoint"])
    trace = model.gradients(case["inputs"], case["targets"])
    close(trace.predictions, case["predictions"])
    close(trace.loss, case["loss"])
    close(trace.weight_gradients, case["weight_gradients"])
    close(trace.bias_gradients, case["bias_gradients"])
    assert model.step == 0
    assert model.train_step(case["inputs"], case["targets"], case["learning_rate"]) == trace
    close(model.to_checkpoint(), case["updated_checkpoint"])
    close(model.predict(case["inputs"]), case["updated_predictions"])
    close(model.gradients(case["inputs"], case["targets"]).loss, case["updated_loss"])


def test_seed_stream_is_language_neutral():
    case = FIXTURE["seed"]
    model = DenseNetwork.seeded(case["widths"], case["activations"], case["value"])
    close(model.to_checkpoint()["layers"][0]["weights"], case["first_weights"])
    assert model.to_checkpoint()["layers"][0]["biases"] == [0, 0]


@pytest.mark.parametrize("kind", ["linear", "tanh", "sigmoid", "relu"])
def test_every_coordinate_matches_finite_differences(kind):
    # Two samples, unequal hidden widths, two outputs: catches accidental MSE
    # normalization by samples only and transposed gradient orientations.
    model = DenseNetwork.seeded([2, 3, 2, 2], [kind, "tanh", "linear"], 42)
    doc = model.to_checkpoint()
    if kind == "relu":
        doc["layers"][0]["biases"] = [0.7, -0.6, 0.8]  # Avoid the kink at zero.
    model = DenseNetwork.from_checkpoint(doc)
    inputs, targets = [[0.2, -0.4], [0.6, 0.3]], [[0.3, -0.5], [-0.1, 0.7]]
    trace = model.gradients(inputs, targets)
    epsilon = 1e-6
    for index, layer in enumerate(doc["layers"]):
        for field in ("weights", "biases"):
            coords = [(r, c) for r, row in enumerate(layer["weights"]) for c in range(len(row))]
            if field == "biases":
                coords = [(0, c) for c in range(len(layer["biases"]))]
            for r, c in coords:
                losses = []
                for direction in (-1, 1):
                    mutated = copy.deepcopy(doc)
                    values = mutated["layers"][index][field]
                    row = values[r] if field == "weights" else values
                    row[c] += direction * epsilon
                    losses.append(
                        DenseNetwork.from_checkpoint(mutated).gradients(inputs, targets).loss
                    )
                expected = (
                    trace.weight_gradients[index][r][c]
                    if field == "weights"
                    else trace.bias_gradients[index][c]
                )
                assert (losses[1] - losses[0]) / (2 * epsilon) == pytest.approx(expected, abs=1e-8)


def test_relu_zero_derivative_and_stable_sigmoid_extremes():
    zero = DenseNetwork([DenseLayer([[1]], [0], "relu")]).gradients([[0]], [[1]])
    assert zero.weight_gradients == [[[0]]]
    assert zero.bias_gradients == [[0]]
    model = DenseNetwork([DenseLayer([[1]], [0], "sigmoid")])
    assert model.predict([[-1000], [1000]]) == [[0], [1]]


def test_seeded_xor_training_and_checkpoint_resume():
    case = FIXTURE["xor"]
    model = DenseNetwork.seeded(case["widths"], case["activations"], case["seed"])
    losses = model.fit(case["inputs"], case["targets"], case["epochs"], case["learning_rate"])
    assert len(losses) == case["epochs"] + 1
    assert model.step == case["epochs"]
    assert losses[-1] < case["maximum_final_loss"] < losses[0]
    predictions = model.predict(case["inputs"])
    assert [int(row[0] >= 0.5) for row in predictions] == [0, 1, 1, 0]
    restored = DenseNetwork.from_checkpoint(
        json.loads(json.dumps(model.to_checkpoint(), allow_nan=False))
    )
    close(restored.predict(case["inputs"]), predictions)
    model.fit(case["inputs"], case["targets"], 2, case["learning_rate"])
    restored.fit(case["inputs"], case["targets"], 2, case["learning_rate"])
    assert model.to_checkpoint() == restored.to_checkpoint()


def test_zero_epochs_one_epoch_and_trace_ownership():
    case = FIXTURE["paper"]
    doc = copy.deepcopy(case["checkpoint"])
    model = DenseNetwork.from_checkpoint(doc)
    doc["layers"][0]["weights"][0][0] = 999
    before = model.to_checkpoint()
    assert model.fit(case["inputs"], case["targets"], 0, 0.01) == [case["loss"]]
    assert model.to_checkpoint() == before
    history = model.fit(case["inputs"], case["targets"], 1, 0.01)
    close(history, [case["loss"], case["updated_loss"]])
    assert model.step == 1
    exported = model.to_checkpoint()
    exported["layers"][0]["biases"][0] = 999
    trace = model.gradients(case["inputs"], case["targets"])
    trace.predictions[0][0] = 999
    trace.weight_gradients[0][0][0] = 999
    close(model.to_checkpoint(), case["updated_checkpoint"])


@pytest.mark.parametrize("value", [True, "1", None, float("nan"), float("inf"), 10**400])
def test_non_numbers_rejected(value):
    with pytest.raises(ValueError):
        DenseNetwork([DenseLayer([[value]], [0])])


@pytest.mark.parametrize(
    "layers",
    [
        None,
        [],
        [None],
        [DenseLayer([], [0])],
        [DenseLayer([[1], [1, 2]], [0])],
        [DenseLayer([[1]], [])],
        [DenseLayer([[1, 2]], [0])],
        [DenseLayer([[1]], [0], "softmax")],
        [DenseLayer([[1]], [0]), DenseLayer([[1], [2]], [0])],
        [DenseLayer([[1]], [True])],
    ],
)
def test_invalid_layers(layers):
    with pytest.raises(ValueError):
        DenseNetwork(layers)


@pytest.mark.parametrize(
    "widths,kinds,seed",
    [
        ([], [], 7),
        ([1], [], 7),
        ([1, 0], ["linear"], 7),
        ([True, 1], ["linear"], 7),
        ([1, 1], [], 7),
        ([1, 1], ["bad"], 7),
        ([1, 1], ["linear"], -1),
        ([1, 1], ["linear"], 2**32),
        ([1, 1], ["linear"], 0.5),
    ],
)
def test_invalid_initialization(widths, kinds, seed):
    with pytest.raises(ValueError):
        DenseNetwork.seeded(widths, kinds, seed)


@pytest.mark.parametrize(
    "inputs,targets",
    [
        ([], [[1]]),
        ([[1]], [[1]]),
        ([[1, 2]], []),
        ([[1, 2]], [[1, 2]]),
        ([[1, 2]], [[1], [2]]),
        ([[True, 2]], [[1]]),
        ([[1, 2]], [[float("nan")]]),
        ([[1e308, 1e308]], [[1]]),
    ],
)
def test_invalid_batch(inputs, targets):
    with pytest.raises(ValueError):
        DenseNetwork.from_checkpoint(FIXTURE["paper"]["checkpoint"]).gradients(inputs, targets)


@pytest.mark.parametrize("rate", [0, -1, True, None, float("nan"), float("inf"), 1e308])
def test_failed_update_is_atomic(rate):
    case = FIXTURE["paper"]
    model = DenseNetwork.from_checkpoint(case["checkpoint"])
    before = model.to_checkpoint()
    with pytest.raises(ValueError):
        model.train_step(case["inputs"], case["targets"], rate)
    assert model.to_checkpoint() == before


@pytest.mark.parametrize("epochs,rate", [(-1, 1), (True, 1), (0.5, 1), (0, 0)])
def test_invalid_fit_controls(epochs, rate):
    with pytest.raises(ValueError):
        DenseNetwork.from_checkpoint(FIXTURE["paper"]["checkpoint"]).fit(
            [[1, 2]], [[1]], epochs, rate
        )


@pytest.mark.parametrize(
    "change",
    [
        lambda doc: doc.update(extra=1),
        lambda doc: doc.update(version=True),
        lambda doc: doc.update(version=2),
        lambda doc: doc.update(format="pickle"),
        lambda doc: doc.update(step=True),
        lambda doc: doc.update(step=-1),
        lambda doc: doc.update(step=2**53),
        lambda doc: doc.update(layers=None),
        lambda doc: doc.update(layers=[]),
        lambda doc: doc["layers"][0].update(extra=1),
        lambda doc: doc["layers"].__setitem__(0, None),
    ],
)
def test_invalid_checkpoint(change):
    doc = copy.deepcopy(FIXTURE["paper"]["checkpoint"])
    change(doc)
    with pytest.raises(ValueError):
        DenseNetwork.from_checkpoint(doc)


def test_checkpoint_requires_object_and_exhausted_step_rejected():
    with pytest.raises(ValueError):
        DenseNetwork.from_checkpoint(None)
    model = DenseNetwork([DenseLayer([[1]], [0])], step=2**53 - 1)
    with pytest.raises(ValueError):
        model.train_step([[1]], [[0]], 0.1)
    assert model.step == 2**53 - 1


def test_finite_candidate_weights_with_overflowing_forward_are_rejected():
    model = DenseNetwork([DenseLayer([[1e100]], [0])])
    before = model.to_checkpoint()
    with pytest.raises(ValueError):
        model.train_step([[1e100]], [[0]], 1)
    assert model.to_checkpoint() == before


def test_finite_loss_with_overflowing_gradient_is_rejected():
    model = DenseNetwork([DenseLayer([[1e-308]], [0])])
    with pytest.raises(ValueError):
        model.gradients([[1e308]], [[0]])
