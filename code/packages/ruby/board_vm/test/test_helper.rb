# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_board_vm"

class FakeRunner
  attr_reader :calls

  def initialize(results = [])
    @results = results
    @calls = []
  end

  def call(argv, chdir: nil)
    @calls << {argv: argv, chdir: chdir}
    @results.shift || CodingAdventures::BoardVM::CommandResult.new(argv, chdir, "", "", 0)
  end
end

class FakeTransport
  attr_reader :frames

  def initialize
    @frames = []
  end

  def transact(frame, timeout_ms:)
    @frames << frame
    "ack".b
  end
end
