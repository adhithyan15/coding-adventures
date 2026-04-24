# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Application
      def self.build(&block)
        new(&block)
      end

      def initialize(&block)
        @router = Router.new
        instance_eval(&block) if block
      end

      def get(pattern, &block)
        @router.add("GET", pattern, &block)
      end

      def post(pattern, &block)
        @router.add("POST", pattern, &block)
      end

      def put(pattern, &block)
        @router.add("PUT", pattern, &block)
      end

      def delete(pattern, &block)
        @router.add("DELETE", pattern, &block)
      end

      def patch(pattern, &block)
        @router.add("PATCH", pattern, &block)
      end

      # Exposed so Rust can iterate routes and register them in WebApp.
      def routes
        @router.routes
      end

      def call(env)
        @router.call(env)
      end
    end

    def self.app(&block)
      Application.build(&block)
    end
  end
end
