# frozen_string_literal: true

require 'minitest/autorun'
require 'stringio'
require 'tempfile'
require_relative '../main'

class NeuralFixtureConsumerTest < Minitest::Test
  FIXTURE = File.expand_path('../../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json', __dir__)

  def test_run_emits_passing_receipt
    output = StringIO.new
    NeuralFixtureConsumer.run(['--fixture', FIXTURE], stdout: output)
    receipt = JSON.parse(output.string)

    assert_equal('ruby-native', receipt.fetch('lane_id'))
    assert_equal([1.0, 0.25], receipt.fetch('contributions'))
    assert_equal(1.35, receipt.fetch('preactivation'))
    assert(receipt.fetch('passes'))
  end

  def test_load_fixture_rejects_unknown_fields
    payload = File.read(FIXTURE).sub('"schema_version": 1,', '"schema_version": 1, "surprise": true,')
    Tempfile.create(['unknown', '.json']) do |file|
      file.write(payload)
      file.flush
      error = assert_raises(ArgumentError) { NeuralFixtureConsumer.evaluate(NeuralFixtureConsumer.load_fixture(file.path)) }
      assert_match(/unexpected keys/, error.message)
    end
  end

  def test_load_fixture_rejects_duplicate_keys
    payload = File.read(FIXTURE).sub('"schema_version": 1,', '"schema_version": 1, "schema_version": 1,')
    Tempfile.create(['duplicate', '.json']) do |file|
      file.write(payload)
      file.flush
      error = assert_raises(JSON::ParserError) { NeuralFixtureConsumer.load_fixture(file.path) }
      assert_match(/duplicate key/, error.message)
    end
  end

  def test_run_requires_exact_arguments
    error = assert_raises(ArgumentError) { NeuralFixtureConsumer.run([], stdout: StringIO.new) }
    assert_match(/usage/, error.message)
  end
end
