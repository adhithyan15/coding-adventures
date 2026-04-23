# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Route
      attr_reader :method, :pattern, :block

      def initialize(method, pattern, &block)
        @method = method
        @pattern = pattern
        @block = block
        @segments = split_segments(pattern)
      end

      def match?(request_method, path)
        return nil unless request_method == method

        path_segments = split_segments(path)
        return nil unless path_segments.length == @segments.length

        params = {}
        @segments.zip(path_segments).each do |expected, actual|
          if expected.start_with?(":")
            params[expected[1..]] = actual
          elsif expected != actual
            return nil
          end
        end
        params
      end

      private

      def split_segments(path)
        return [] if path == "/"

        path.split("/").reject(&:empty?)
      end
    end
  end
end
