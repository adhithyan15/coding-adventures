# frozen_string_literal: true

require "rbconfig"

module CodingAdventures
  module MiniRedisNative
    class Server
      DEFAULT_HOST = "127.0.0.1"
      DEFAULT_MAX_CONNECTIONS = 64
      DEFAULT_WORKER_PROCESSES = 1
      DEFAULT_WORKER_QUEUE_DEPTH = 1024
      DEFAULT_WORKER_PROGRAM = RbConfig.ruby
      DEFAULT_WORKER_ARGS = [File.expand_path("stdio_worker.rb", __dir__)].freeze

      attr_reader :host

      def initialize(
        host: DEFAULT_HOST,
        port: 0,
        max_connections: DEFAULT_MAX_CONNECTIONS,
        worker_processes: DEFAULT_WORKER_PROCESSES,
        worker_queue_depth: DEFAULT_WORKER_QUEUE_DEPTH,
        worker_program: DEFAULT_WORKER_PROGRAM,
        worker_args: DEFAULT_WORKER_ARGS
      )
        @native = NativeServer.new(
          host,
          Integer(port),
          Integer(max_connections),
          Integer(worker_processes),
          Integer(worker_queue_depth),
          worker_program,
          worker_args
        )
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
