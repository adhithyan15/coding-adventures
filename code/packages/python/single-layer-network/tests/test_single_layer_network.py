from single_layer_network import SingleLayerNetwork, fit_single_layer_network, train_one_epoch_with_matrices


def near(actual, expected, epsilon=1e-6):
    assert abs(actual - expected) <= epsilon


def test_one_epoch_exposes_matrix_gradients():
    step = train_one_epoch_with_matrices(
        inputs=[[1.0, 2.0]],
        targets=[[3.0, 5.0]],
        weights=[[0.0, 0.0], [0.0, 0.0]],
        biases=[0.0, 0.0],
        learning_rate=0.1,
    )

    assert step.predictions == [[0.0, 0.0]]
    assert step.errors == [[-3.0, -5.0]]
    assert step.weight_gradients == [[-3.0, -5.0], [-6.0, -10.0]]
    assert step.bias_gradients == [-3.0, -5.0]
    near(step.next_weights[0][0], 0.3)
    near(step.next_weights[0][1], 0.5)
    near(step.next_weights[1][0], 0.6)
    near(step.next_weights[1][1], 1.0)
    near(step.next_biases[0], 0.3)
    near(step.next_biases[1], 0.5)


def test_fit_learns_three_inputs_to_two_outputs():
    inputs = [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]]
    targets = [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]]
    model = SingleLayerNetwork.with_shape(3, 2)
    history = model.fit(inputs, targets, learning_rate=0.05, epochs=500)
    assert history[-1].loss < history[0].loss
    prediction = model.predict([[1.0, 1.0, 1.0]])[0]
    assert len(prediction) == 2


def test_fit_helper_infers_shape():
    model = fit_single_layer_network([[0.0], [1.0], [2.0]], [[0.0, 0.0], [1.0, 2.0], [2.0, 4.0]], epochs=400)
    prediction = model.predict([[3.0]])[0]
    near(prediction[0], 3.0, 0.2)
    near(prediction[1], 6.0, 0.4)
