# frozen_string_literal: true

require "json"
require "uri"

module CodingAdventures
  module Conduit
    class Request
      attr_reader :env, :params, :query_params, :headers

      def initialize(env, params: {}, query_params: {}, headers: {})
        @env = env
        @params = params
        @query_params = query_params
        @headers = headers
      end

      def method
        env.fetch("REQUEST_METHOD")
      end

      def path
        env.fetch("PATH_INFO")
      end

      def query_string
        env.fetch("QUERY_STRING", "")
      end

      # Raw request body as a String. When rack.input is an IO-like object
      # (e.g. a StringIO), it is rewound and read once; subsequent calls
      # return the same string via memoization.
      def body
        @body ||= begin
          input = env.fetch("rack.input", "")
          if input.is_a?(String)
            input
          else
            input.rewind if input.respond_to?(:rewind)
            input.read.to_s
          end
        end
      end

      # Parse the request body as JSON. Memoized. Raises JSON::ParserError on
      # invalid input. Useful for Content-Type: application/json requests.
      def json
        @parsed_json ||= JSON.parse(body)
      end

      # Parse the request body as URL-encoded form data (application/x-www-form-urlencoded).
      # Returns a Hash. Memoized.
      def form
        @parsed_form ||= URI.decode_www_form(body).to_h
      end

      def header(name)
        headers[name.to_s.downcase]
      end

      def content_length
        value = env["conduit.content_length"]
        return nil if value.nil?

        Integer(value)
      end

      def content_type
        env["conduit.content_type"]
      end

      def [](key)
        env[key]
      end
    end
  end
end
