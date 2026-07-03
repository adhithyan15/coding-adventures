# frozen_string_literal: true

module CodingAdventures
  module IrcServerNative
    # IrcServer — the Ruby control surface for the Rust IRC engine.
    #
    # A thin, ergonomic wrapper over the native +NativeServer+ class, which
    # embeds the all-Rust +irc-net-reactor+ engine.  Every line of IRC and TCP
    # logic runs in Rust; this class only creates, runs, and stops the server.
    #
    # Typical usage:
    #
    #   server = CodingAdventures::IrcServerNative::IrcServer.new(port: 6667)
    #   server.serve            # blocks until another thread calls #stop
    #
    # Or run it in the background and stop after assertions (e.g. in tests):
    #
    #   server = CodingAdventures::IrcServerNative::IrcServer.new(port: 0)
    #   server.start            # serves on a background thread
    #   # ... connect IRC clients to server.local_host:server.local_port ...
    #   server.close
    class IrcServer
      DEFAULT_HOST = "127.0.0.1"
      DEFAULT_PORT = 6667
      DEFAULT_SERVER_NAME = "irc.local"
      DEFAULT_MOTD = ["Welcome."].freeze
      DEFAULT_MAX_CONNECTIONS = 1024

      def initialize(host: DEFAULT_HOST, port: DEFAULT_PORT, server_name: DEFAULT_SERVER_NAME,
                     motd: nil, oper_password: "", max_connections: DEFAULT_MAX_CONNECTIONS)
        motd_lines = (motd && !motd.empty? ? motd : DEFAULT_MOTD).map(&:to_s)
        @native = NativeServer.new(
          host.to_s,
          Integer(port),
          server_name.to_s,
          motd_lines,
          oper_password.to_s,
          Integer(max_connections)
        )
        @thread = nil
        @closed = false
      end

      # Run the event loop, blocking until #stop is called.  The native layer
      # releases the GVL, so another Ruby thread can call #stop.
      def serve
        ensure_open
        @native.serve
      end

      # Serve on a background thread and return once the loop is running.
      def start
        ensure_open
        raise Error, "server thread is already running" if @thread&.alive?

        @thread = Thread.new { @native.serve }
        wait_until_running
        @thread
      end

      # Signal the event loop to stop; a blocked #serve returns.
      def stop
        return if @closed

        @native.stop
      end

      # Join the background serve thread (if any).
      def wait(timeout = nil)
        @thread&.join(timeout)
      end

      # Stop the server, wait for the background thread, and release the listener.
      def close
        return if @closed

        stop
        wait(5)
        @native.dispose
        @closed = true
      end

      def running?
        !@closed && @native.running?
      end

      def local_host
        @native.local_host
      end

      def local_port
        @native.local_port
      end

      # The bound "host:port" address.
      def local_addr
        "#{local_host}:#{local_port}"
      end

      private

      def ensure_open
        raise Error, "server is closed" if @closed
      end

      def wait_until_running
        100.times do
          return if @native.running?
          raise Error, "server thread exited before listening" unless @thread.alive?

          sleep 0.01
        end
        raise Error, "server did not start listening in time"
      end
    end
  end
end
