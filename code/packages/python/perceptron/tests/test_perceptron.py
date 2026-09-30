import pytest

from perceptron import Perceptron


def test_constructor_preserves_training_configuration() -> None:
    model = Perceptron(learning_rate=0.25, epochs=12)
    assert model.lr == 0.25
    assert model.epochs == 12
    assert model.weights is None
    assert model.bias == 0.0


def test_predict_before_fit_raises() -> None:
    with pytest.raises(ValueError, match="has not been trained"):
        Perceptron().predict([[0.0, 0.0]])


def test_fit_learns_and_gate_with_bounded_logging(capsys) -> None:
    features = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    labels = [[0.0], [0.0], [0.0], [1.0]]
    model = Perceptron(learning_rate=0.8, epochs=5000)

    model.fit(features, labels, log_steps=5001)

    assert model.weights is not None
    assert (model.weights.rows, model.weights.cols) == (2, 1)
    predictions = model.predict(features)
    assert len(predictions) == len(labels)
    assert all(0.0 <= prediction <= 1.0 for prediction in predictions)
    assert all(prediction < 0.2 for prediction in predictions[:3])
    assert predictions[3] > 0.8
    output = capsys.readouterr().out
    assert output.count("Epoch") == 1
