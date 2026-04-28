require "minitest/autorun"
require_relative "../lib/single_layer_network"

class SingleLayerNetworkTest < Minitest::Test
  def assert_near(expected, actual)
    assert_in_delta expected, actual, 1e-6
  end

  def test_one_epoch_exposes_matrix_gradients
    step = SingleLayerNetwork.train_one_epoch_with_matrices(
      [[1.0, 2.0]],
      [[3.0, 5.0]],
      [[0.0, 0.0], [0.0, 0.0]],
      [0.0, 0.0],
      0.1
    )

    assert_equal [[0.0, 0.0]], step.predictions
    assert_equal [[-3.0, -5.0]], step.errors
    assert_equal [[-3.0, -5.0], [-6.0, -10.0]], step.weight_gradients
    assert_near 0.3, step.next_weights[0][0]
    assert_near 1.0, step.next_weights[1][1]
  end

  def test_fit_learns_m_inputs_to_n_outputs
    model = SingleLayerNetwork::Model.new(input_count: 3, output_count: 2)
    history = model.fit(
      [[0.0, 0.0, 1.0], [1.0, 2.0, 1.0], [2.0, 1.0, 1.0]],
      [[1.0, -1.0], [3.0, 2.0], [4.0, 1.0]],
      learning_rate: 0.05,
      epochs: 500
    )
    assert_operator history.last.loss, :<, history.first.loss
    assert_equal 2, model.predict([[1.0, 1.0, 1.0]]).first.length
  end
end
