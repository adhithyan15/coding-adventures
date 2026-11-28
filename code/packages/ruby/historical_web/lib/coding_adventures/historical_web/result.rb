# frozen_string_literal: true

module CodingAdventures
  module HistoricalWeb
    # Expected boundary failures are values rather than exceptions. A success
    # carries `value`; a failure carries one of the documented error symbols.
    class Result
      attr_reader :value, :error

      def self.success(value)
        new(true, value, nil)
      end

      def self.failure(error)
        new(false, nil, error)
      end

      def ok?
        @ok
      end

      private

      def initialize(ok, value, error)
        @ok = ok
        @value = value
        @error = error
        freeze
      end
    end
  end
end
