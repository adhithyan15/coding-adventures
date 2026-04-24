# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Application
      attr_reader :before_filters, :after_filters, :not_found_handler, :error_handler, :settings

      def self.build(&block)
        new(&block)
      end

      def initialize(&block)
        @router = Router.new
        @before_filters = []
        @after_filters = []
        @not_found_handler = nil
        @error_handler = nil
        @settings = {}
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

      # Register a before filter. Runs for every request, before route lookup.
      # Matches Sinatra semantics — fires even when no route matches, so it can
      # implement maintenance mode, auth, or rate limiting on all paths.
      # Named route params are not available; use request.path / query_params.
      # Call halt() to short-circuit and send a response immediately.
      def before(&block)
        @before_filters << block
      end

      # Register an after filter. Runs after every route handler for side effects
      # such as logging or metrics. Response modification is not supported yet.
      def after(&block)
        @after_filters << block
      end

      # Register a custom not-found handler. Called when no route matches.
      def not_found(&block)
        @not_found_handler = block
      end

      # Register a custom error handler. Called when a route handler raises.
      # The block receives (request, error_message).
      def error(&block)
        @error_handler = block
      end

      # Store a configuration value (e.g. set :port, 3000).
      def set(key, value)
        @settings[key.to_sym] = value
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
