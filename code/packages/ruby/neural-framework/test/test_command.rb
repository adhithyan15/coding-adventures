# frozen_string_literal: true

require_relative "test_helper"
require "stringio"
require_relative "../examples/xor"

class CommandTest < Minitest::Test
  include FixtureAssertions

  def test_actual_native_receipts_and_resume
    example = NF::XORExample
    output = StringIO.new
    example.run([], StringIO.new, output)
    receipt = JSON.parse(output.string)
    assert_equal 3000, receipt["checkpoint"]["step"]
    assert_operator receipt["final_loss"], :<, 0.01
    model = NET.from_checkpoint(receipt["checkpoint"])
    2.times do |index|
      output = StringIO.new
      example.run(["--resume"], StringIO.new(JSON.generate(receipt["checkpoint"])), output)
      receipt = JSON.parse(output.string)
      model.train_step(example::INPUTS, example::TARGETS, 1)
      assert_equal 3001 + index, receipt["checkpoint"]["step"]
      close_data(receipt["checkpoint"], model.to_checkpoint)
      close_data(receipt["predictions"], model.predict(example::INPUTS))
    end
  end

  def test_command_rejects_arguments_and_oversized_stdin
    assert_raises(ArgumentError) { NF::XORExample.run(["--unknown"], StringIO.new, StringIO.new) }
    input = Object.new
    input.define_singleton_method(:read) do |limit|
      raise "unbounded stdin" unless limit == CodingAdventures::NeuralFramework::MAX_JSON_BYTES + 1
      " " * limit
    end
    assert_raises(ArgumentError) { NF::XORExample.run(["--resume"], input, StringIO.new) }
  end
end
