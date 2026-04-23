# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class NativeServer
      def attach_app(app)
        @app = app
        self
      end

      def dispatch_request(env)
        raise ServerError, "no app attached" unless @app

        status, headers, body = @app.call(env)
        [
          Integer(status),
          normalize_headers(headers),
          normalize_body(body)
        ]
      end

      private

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
        @native = NativeServer.new(host, Integer(port), Integer(max_connections))
        @native.attach_app(app)
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
