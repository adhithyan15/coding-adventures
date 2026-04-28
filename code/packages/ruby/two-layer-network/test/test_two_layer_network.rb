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

  def test_hidden_layer_teaching_examples_run_one_training_step
    cases = [
      ["XNOR", XOR_INPUTS, [[1.0], [0.0], [0.0], [1.0]], 3],
      ["absolute value", [[-1.0], [-0.5], [0.0], [0.5], [1.0]], [[1.0], [0.5], [0.0], [0.5], [1.0]], 4],
      ["piecewise pricing", [[0.1], [0.3], [0.5], [0.7], [0.9]], [[0.12], [0.25], [0.55], [0.88], [0.88]], 4],
      ["circle classifier", [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0], [-0.5, 0.5], [-1.0, 0.0]], [[1.0], [1.0], [0.0], [1.0], [0.0]], 5],
      ["two moons", [[1.0, 0.0], [0.0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1.0, 0.0], [2.0, 0.5]], [[0.0], [1.0], [0.0], [1.0], [0.0], [1.0]], 5],
      ["interaction features", [[0.2, 0.25, 0.0], [0.6, 0.5, 1.0], [1.0, 0.75, 1.0], [1.0, 1.0, 0.0]], [[0.08], [0.72], [0.96], [0.76]], 5]
    ]

    cases.each do |name, inputs, targets, hidden_count|
      step = TwoLayerNetwork.train_one_epoch(inputs, targets, sample_parameters(inputs.first.length, hidden_count), 0.4)

      assert_operator step.loss, :>=, 0.0, name
      assert_equal inputs.first.length, step.input_to_hidden_weight_gradients.length, name
      assert_equal hidden_count, step.hidden_to_output_weight_gradients.length, name
    end
  end

  private

  def sample_parameters(input_count, hidden_count)
    TwoLayerNetwork::Parameters.new(
      input_to_hidden_weights: Array.new(input_count) do |feature|
        Array.new(hidden_count) { |hidden| 0.17 * (feature + 1) - 0.11 * (hidden + 1) }
      end,
      hidden_biases: Array.new(hidden_count) { |hidden| 0.05 * (hidden - 1) },
      hidden_to_output_weights: Array.new(hidden_count) { |hidden| [0.13 * (hidden + 1) - 0.25] },
      output_biases: [0.02]
    )
  end
end
