require "minitest/autorun"
require_relative "../lib/two_layer_network"

class TwoLayerNetworkTest < Minitest::Test
  XOR_INPUTS = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
  XOR_TARGETS = [[0.0], [1.0], [1.0], [0.0]]

  def test_forward_pass_exposes_hidden_activations
    passed = TwoLayerNetwork.forward(XOR_INPUTS, TwoLayerNetwork.xor_warm_start_parameters)

    assert_equal 4, passed.hidden_activations.length
    assert_equal 2, passed.hidden_activations.first.length
    assert_operator passed.predictions[1][0], :>, 0.7
    assert_operator passed.predictions[0][0], :<, 0.3
  end

  def test_training_step_exposes_both_layer_gradients
    step = TwoLayerNetwork.train_one_epoch(
      XOR_INPUTS,
      XOR_TARGETS,
      TwoLayerNetwork.xor_warm_start_parameters,
      0.5
    )

    assert_equal 2, step.input_to_hidden_weight_gradients.length
    assert_equal 2, step.input_to_hidden_weight_gradients.first.length
    assert_equal 2, step.hidden_to_output_weight_gradients.length
    assert_equal 1, step.hidden_to_output_weight_gradients.first.length
  end
end
