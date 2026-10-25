# frozen_string_literal: true

require_relative "test_helper"

class BoundaryTest < Minitest::Test
  include FixtureAssertions

  def test_admission_and_shapes
    [nil, true, "1", Float::NAN, Float::INFINITY, Complex(1, 2), 10**1000].each do |bad|
      assert_raises(ArgumentError) { linear(bad) }
      assert_raises(ArgumentError) { linear.predict([[bad]]) }
    end
    [[], nil, [nil]].each { |bad| assert_raises(ArgumentError) { NET.new(bad) } }
    [[], [[1], []], [[1, 2]], nil].each { |bad| assert_raises(ArgumentError) { linear.predict(bad) } }
    assert_raises(ArgumentError) { linear.gradients([[1]], [[1, 2]]) }
    assert_raises(ArgumentError) { NET.new([LAYER.new(weights: [[1]], biases: [1, 2], activation: "linear")]) }
    assert_raises(ArgumentError) { NET.new([LAYER.new(weights: [[1]], biases: [1], activation: "unknown")]) }
    assert_raises(ArgumentError) { NET.new([LAYER.new(weights: [[1]], biases: [0], activation: "linear"), LAYER.new(weights: [[1], [2]], biases: [0], activation: "linear")]) }
  end

  def test_counts_rates_and_capacity
    [nil, [], [1], [0, 1], [1.0, 1], [true, 1]].each { |widths| assert_raises(ArgumentError) { NET.seeded(widths, ["linear"]) } }
    [nil, [], ["bad"]].each { |acts| assert_raises(ArgumentError) { NET.seeded([1, 1], acts) } }
    [-1, 1.0, true, 1 << 32].each { |seed| assert_raises(ArgumentError) { NET.seeded([1, 1], ["linear"], seed: seed) } }
    assert_raises(ArgumentError) { NET.seeded([NF::MAX_STEP, NF::MAX_STEP], ["linear"]) }
    model = linear
    before = model.to_checkpoint
    [0, -1, true, Float::INFINITY].each { |rate| assert_raises(ArgumentError) { model.train_step([[1]], [[0]], rate) } }
    [-1, 0.5, true, NF::MAX_STEP + 1].each { |count| assert_raises(ArgumentError) { model.fit([[1]], [[0]], count, 1) } }
    assert_equal before, model.to_checkpoint
    before["step"] = NF::MAX_STEP
    exhausted = NET.from_checkpoint(before)
    assert_raises(ArgumentError) { exhausted.train_step([[1]], [[0]], 1) }
    assert_raises(ArgumentError) { exhausted.fit([[1]], [[0]], 1, 1) }
    assert_equal before, exhausted.to_checkpoint
    refute_respond_to exhausted, :step=
  end

  def test_failed_updates_are_atomic
    [[Float::MAX, [[1]], [[0]]], [1e154, [[1]], [[0]]]].each do |rate, inputs, targets|
      model = linear
      before = model.to_checkpoint
      assert_raises(ArgumentError) { model.train_step(inputs, targets, rate) }
      assert_equal before, model.to_checkpoint
    end
    assert_raises(ArgumentError) { linear(Float::MAX).predict([[2]]) }
    assert_raises(ArgumentError) { linear.gradients([[Float::MAX]], [[-Float::MAX]]) }
  end

  def test_all_public_arrays_are_owned
    weights = [[2]]
    biases = [1]
    activation = +"linear"
    model = NET.new([LAYER.new(weights: weights, biases: biases, activation: activation)])
    weights[0][0] = 90
    biases[0] = 80
    activation.replace("relu")
    expected = model.to_checkpoint
    input = [[3].freeze].freeze
    assert_equal [[7.0]], model.predict(input)
    model.predict(input)[0][0] = 100
    trace = model.gradients(input, [[0]])
    trace.predictions[0][0] = 100
    trace.weight_gradients[0][0][0] = 100
    trace.bias_gradients[0][0] = 100
    checkpoint = model.to_checkpoint
    loaded = NET.from_checkpoint(checkpoint)
    checkpoint["layers"][0]["weights"][0][0] = 100
    checkpoint["layers"][0]["biases"][0] = 100
    checkpoint["layers"][0]["activation"].replace("relu")
    assert_equal expected, model.to_checkpoint
    assert_equal expected, loaded.to_checkpoint
  end

  def test_json_envelope
    [nil, "{} {}", "{", "NaN", "[" * 130 + "0" + "]" * 130, " " * (NF::MAX_JSON_BYTES + 1)].each do |text|
      assert_raises(ArgumentError) { NET.from_json(text) }
    end
    assert_equal linear.to_checkpoint, NET.from_json(linear.to_json).to_checkpoint
  end
end
