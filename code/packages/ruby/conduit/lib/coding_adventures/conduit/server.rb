# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class NativeServer
      # Called by Rust when a route match is found. `route_index` is the
      # position in app.routes; `env` is the Rack-compatible env hash
      # (already enriched with conduit.route_params by the Rust router).
      def native_dispatch_route(route_index, env)
        route = @app.routes[route_index]
        request = Request.new(
          env,
          params: env.fetch("conduit.route_params", {}),
          query_params: env.fetch("conduit.query_params", {}),
          headers: env.fetch("conduit.headers", {})
        )
        result = invoke_route(route.block, request)
        normalize_result(result)
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
          [Integer(result[0]), normalize_headers(result[1]), normalize_body(result[2])]
        when String
          [200, [["content-type", "text/plain; charset=utf-8"]], [result]]
        else
          [200, [["content-type", "text/plain; charset=utf-8"]], [result.to_s]]
        end
      end

      def normalize_headers(headers)
        return [] if headers.nil?

        headers.to_h.map do |key, value|
          [String(key), String(value)]
        end
      end

      def normalize_body(body)
        return [] if body.nil?
        return [body] if body.is_a?(String)

        if body.respond_to?(:each)
          chunks = []
          body.each { |chunk| chunks << String(chunk) }
          body.close if body.respond_to?(:close)
          chunks
        else
          [String(body)]
        end
      end
    end

    class Server
      DEFAULT_HOST = "127.0.0.1"
      DEFAULT_MAX_CONNECTIONS = 1024

      attr_reader :host

      def initialize(app, host: DEFAULT_HOST, port: 0, max_connections: DEFAULT_MAX_CONNECTIONS)
        @app = app
        @native = NativeServer.new(app, host, Integer(port), Integer(max_connections))
        @native.instance_variable_set(:@app, app)
        @host = @native.local_host
        @thread = nil
        @closed = false
      end

      def port
        ensure_open
        @native.local_port
      end

      def local_addr
        "#{host}:#{port}"
      end

      def running?
        !@closed && @native.running?
      end

      def serve
        ensure_open
        @native.serve
      end

      def start
        ensure_open
        raise ServerError, "server thread is already running" if @thread&.alive?

        @thread = Thread.new { @native.serve }
        wait_until_running
        @thread
      end

      def stop
        return if @closed

        @native.stop
      end

      def wait(timeout = nil)
        @thread&.join(timeout)
      end

      def close
        return if @closed

        stop
        wait(5)
        @native.dispose
        @closed = true
      end

      private

      def ensure_open
        raise ServerError, "server is closed" if @closed
      end

      def wait_until_running
        100.times do
          return if running?
          raise ServerError, "server thread exited before listening" unless @thread.alive?

          sleep 0.01
        end
      end
    end
  end
end
