# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Route
      attr_reader :method, :pattern, :block

      def initialize(method, pattern, &block)
        @method = method
        @pattern = pattern
        @block = block
      end

      def match?(request_method, path)
        return nil unless request_method == method
        if use_native_matcher?
          CodingAdventures::Conduit.match_route_native(pattern, path)
        else
          fallback_match(path)
        end
      end

      private

      def fallback_match(path)
        pattern_segments = split_segments(pattern)
        path_segments = split_segments(path)
        return nil unless path_segments.length == pattern_segments.length

        params = {}
        pattern_segments.zip(path_segments).each do |expected, actual|
          if expected.start_with?(":")
            params[expected[1..]] = actual
          elsif expected != actual
            return nil
          end
        end
        params
      end

      def split_segments(path)
        return [] if path == "/"

        path.split("/").reject(&:empty?)
      end

      def use_native_matcher?
        return false unless CodingAdventures::Conduit.respond_to?(:match_route_native)

        # The native route matcher is safe on the direct Ruby path, but the
        # embedded server callback currently re-enters Ruby from a native
        # reactor thread. Keep that path on the pure Ruby matcher until the
        # cross-thread bridge is hardened.
        Thread.current == Thread.main
      end
    end
  end
end
