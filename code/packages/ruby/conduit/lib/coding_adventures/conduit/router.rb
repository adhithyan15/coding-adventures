# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Router
      attr_reader :routes

      def initialize
        @routes = []
      end

      def add(method, pattern, &block)
        @routes << Route.new(method, pattern, &block)
      end

      def call(env)
        method = env.fetch("REQUEST_METHOD")
        path = env.fetch("PATH_INFO")
        query_params = env.fetch("conduit.query_params", {})
        headers = env.fetch("conduit.headers", {})

        @routes.each do |route|
          params = route.match?(method, path)
          next if params.nil?

          request = Request.new(
            env,
            params: params,
            query_params: query_params,
            headers: headers
          )
          return normalize_result(invoke_route(route.block, request))
        end

        not_found
      end

      private

      def invoke_route(block, request)
        case block.arity
        when 0
          request.instance_exec(&block)
        else
          block.call(request)
        end
      end

      def normalize_result(result)
        case result
        when Array
          result
        when String
          [200, { "content-type" => "text/plain" }, [result]]
        else
          [200, { "content-type" => "text/plain" }, [result.to_s]]
        end
      end

      def not_found
        [404, { "content-type" => "text/plain" }, ["Not Found"]]
      end
    end
  end
end
