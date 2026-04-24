# frozen_string_literal: true

module CodingAdventures
  module Conduit
    # Raised by HandlerContext helpers (halt, redirect, json, html, text) to
    # immediately send a response without unwinding through normal return.
    #
    # Never leaks outside the dispatch boundary — native_dispatch_route and
    # native_run_before_filters both catch it and convert it to a Rack triplet.
    #
    #   halt(403, "Forbidden")
    #   json({ error: "bad" }, 422)
    #   redirect "/login"
    class HaltError < StandardError
      attr_reader :status, :body, :halt_headers

      def initialize(status, body = "", headers = {})
        super("halt #{status}")
        @status = Integer(status)
        @body = body.to_s
        @halt_headers = normalize_headers(headers)
      end

      private

      def normalize_headers(headers)
        return [] if headers.nil? || (headers.respond_to?(:empty?) && headers.empty?)

        if headers.is_a?(Hash)
          headers.map { |k, v| [k.to_s, v.to_s] }
        else
          Array(headers).map { |pair| [pair[0].to_s, pair[1].to_s] }
        end
      end
    end
  end
end
