# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  root File.expand_path("..", __dir__)
  track_files "lib/**/*.rb"
  add_filter "/test/"
  add_filter "/scripts/"
  add_filter "/examples/"
  minimum_coverage 95
end
require "minitest/autorun"
require "neural_framework"

module FixtureAssertions
  NF = CodingAdventures::NeuralFramework
  NET = NF::DenseNetwork
  LAYER = NF::DenseLayer
  FIXTURES = File.expand_path("../../../../specs/fixtures/neural-framework-v1", __dir__)

  def fixture(name)
    JSON.parse(File.read(File.join(FIXTURES, name)))
  end

  def close_data(actual, expected, tolerance = 1e-9)
    if expected.is_a?(Numeric)
      assert_in_delta expected, actual, tolerance
    elsif expected.is_a?(Array)
      assert_equal expected.length, actual.length
      expected.zip(actual).each { |e, a| close_data(a, e, tolerance) }
    elsif expected.is_a?(Hash)
      assert_equal expected.keys.sort, actual.keys.sort
      expected.each { |key, value| close_data(actual[key], value, tolerance) }
    else
      assert_equal expected, actual
    end
  end

  def linear(weight = 1.0, bias = 0.0, activation = "linear")
    NET.new([LAYER.new(weights: [[weight]], biases: [bias], activation: activation)])
  end
end
