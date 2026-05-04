from two_layer_network import (
    TwoLayerNetwork,
    TwoLayerParameters,
    create_xor_warm_start_parameters,
    forward_two_layer,
    train_one_epoch_two_layer,
)

XOR_INPUTS = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
XOR_TARGETS = [[0.0], [1.0], [1.0], [0.0]]


def test_forward_pass_exposes_hidden_activations():
    passed = forward_two_layer(XOR_INPUTS, create_xor_warm_start_parameters())

    assert len(passed.hidden_activations) == 4
    assert len(passed.hidden_activations[0]) == 2
    assert passed.predictions[1][0] > 0.7
    assert passed.predictions[0][0] < 0.3


def test_training_step_exposes_both_layer_gradients():
    step = train_one_epoch_two_layer(
        XOR_INPUTS,
        XOR_TARGETS,
        create_xor_warm_start_parameters(),
        0.5,
    )

    assert len(step.input_to_hidden_weight_gradients) == 2
    assert len(step.input_to_hidden_weight_gradients[0]) == 2
    assert len(step.hidden_to_output_weight_gradients) == 2
    assert len(step.hidden_to_output_weight_gradients[0]) == 1


def test_warm_start_solves_xor():
    network = TwoLayerNetwork(create_xor_warm_start_parameters(), learning_rate=0.5)
    predictions = [row[0] for row in network.predict(XOR_INPUTS)]

    assert predictions[0] < 0.2
    assert predictions[1] > 0.7
    assert predictions[2] > 0.7
    assert predictions[3] < 0.2


def _sample_parameters(input_count: int, hidden_count: int) -> TwoLayerParameters:
    return TwoLayerParameters(
        input_to_hidden_weights=[
            [0.17 * (feature + 1) - 0.11 * (hidden + 1) for hidden in range(hidden_count)]
            for feature in range(input_count)
        ],
        hidden_biases=[0.05 * (hidden - 1) for hidden in range(hidden_count)],
        hidden_to_output_weights=[[0.13 * (hidden + 1) - 0.25] for hidden in range(hidden_count)],
        output_biases=[0.02],
    )


def test_hidden_layer_teaching_examples_run_one_training_step():
    cases = [
        ("XNOR", XOR_INPUTS, [[1.0], [0.0], [0.0], [1.0]], 3),
        ("absolute value", [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4),
        ("piecewise pricing", [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4),
        ("circle classifier", [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5),
        ("two moons", [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5),
        ("interaction features", [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5),
    ]

    for name, inputs, targets, hidden_count in cases:
        step = train_one_epoch_two_layer(inputs, targets, _sample_parameters(len(inputs[0]), hidden_count), 0.4)

        assert step.loss >= 0.0, name
        assert len(step.input_to_hidden_weight_gradients) == len(inputs[0]), name
        assert len(step.hidden_to_output_weight_gradients) == hidden_count, name
