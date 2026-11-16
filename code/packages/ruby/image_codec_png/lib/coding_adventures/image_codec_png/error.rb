# frozen_string_literal: true

require_relative "constants"
require_relative "messages"

module CodingAdventures
  module ImageCodecPng
    # Stable, payload-blind IC18 PNG failure.
    # Standing Rule #20: 100% of exception messages are structurally derived from Messages.
    class PngError < StandardError
      attr_reader :code

      def initialize(code)
        unless Constants::PNG_ERROR_CODES.include?(code)
          @code = Constants::ERROR_UNKNOWN
          super(Messages::UNKNOWN_ERROR_MESSAGE)
          return
        end

        @code = code
        super(Messages.message_for(code))
      end

      def ==(other)
        other.is_a?(PngError) && other.code == @code
      end
      alias_method :eql?, :==

      def hash
        [self.class, @code].hash
      end
    end
  end
end
