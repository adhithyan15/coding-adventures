# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Request
      attr_reader :env, :params

      def initialize(env, params: {})
        @env = env
        @params = params
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

      def [](key)
        env[key]
      end
    end
  end
end
