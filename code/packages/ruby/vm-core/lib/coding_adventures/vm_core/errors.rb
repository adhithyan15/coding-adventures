# frozen_string_literal: true

module CodingAdventures
  module VmCore
    class VMError < StandardError; end
    class UnknownOpcodeError < VMError; end
    class FrameOverflowError < VMError; end
    class VMInterrupt < VMError; end
  end
end
