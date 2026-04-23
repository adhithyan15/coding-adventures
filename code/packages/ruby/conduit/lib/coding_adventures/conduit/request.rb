# frozen_string_literal: true

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

      def body
        env.fetch("rack.input", "")
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
