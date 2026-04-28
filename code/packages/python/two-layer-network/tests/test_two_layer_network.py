from two_layer_network import (
    TwoLayerNetwork,
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
