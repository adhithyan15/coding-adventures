# frozen_string_literal: true

require "minitest/autorun"
require "stringio"
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

class FakeWriteTransport
  attr_reader :frames

  def initialize
    @frames = []
  end

  def write(frame)
    @frames << frame
  end
end

class FakeTransactTransport
  attr_reader :frames, :timeout_values

  def initialize(responses = [])
    @responses = responses
    @frames = []
    @timeout_values = []
  end

  def transact(frame, timeout_ms:)
    @frames << frame
    @timeout_values << timeout_ms
    @responses.shift
  end
end

class FakeReadAfterTransactTransport
  attr_reader :frames, :timeout_values, :read_timeout_values

  def initialize(responses)
    @responses = responses
    @frames = []
    @timeout_values = []
    @read_timeout_values = []
  end

  def transact(frame, timeout_ms:)
    @frames << frame
    @timeout_values << timeout_ms
    @responses.shift
  end

  def read(timeout_ms:)
    @read_timeout_values << timeout_ms
    @responses.shift
  end
end

class FakeDecodedSession
  def initialize(decoded_by_response)
    @decoded_by_response = decoded_by_response
  end

  def decode_response(response)
    @decoded_by_response.fetch(response)
  end
end

class FakeTimeoutTransport
  attr_reader :frames, :timeout_values

  def initialize(timeout_at:)
    @timeout_at = timeout_at
    @frames = []
    @timeout_values = []
    @closed = false
  end

  def transact(frame, timeout_ms:)
    @frames << frame
    @timeout_values << timeout_ms
    return nil unless @frames.length == @timeout_at

    raise CodingAdventures::BoardVM::TransportError,
      "timed out waiting for Board VM response on fake transport"
  end

  def close
    @closed = true
  end

  def closed?
    @closed
  end
end
