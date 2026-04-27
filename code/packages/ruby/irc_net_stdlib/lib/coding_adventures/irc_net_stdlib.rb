# frozen_string_literal: true

# irc_net_stdlib — Level 1 network implementation: stdlib sockets + threads.
#
# == Overview
#
# This package provides the *concrete TCP networking layer* for the IRC stack.
# It uses Ruby's +TCPServer+, +Thread+, and +Mutex+ from the standard library.
# Each accepted TCP connection gets its own OS thread (the "thread-per-connection"
# model), which makes the code easy to read and reason about.
#
# == Thread-per-connection model
#
# For each accepted connection, a daemon thread is spawned.  The thread:
# 1. Calls +handler.on_connect+ to notify the server.
# 2. Loops calling +conn.read+ (blocking +recv+) and forwards each chunk
#    to +handler.on_data+.
# 3. When +recv+ returns <tt>""</tt> (peer closed), calls
#    +handler.on_disconnect+ and exits.
#
# This is the textbook model taught in every OS/networking course.  Its chief
# virtue is clarity: each connection's lifecycle is a simple sequential program.
#
# == Two mutexes protect shared state
#
# *+@handler_mutex+* (a +Mutex+):
#   Serialises *all* calls to the +Handler+ (+on_connect+, +on_data+,
#   +on_disconnect+).  The Handler's internals (i.e. the IRC server state
#   machine) are *not* thread-safe.  By funnelling every callback through
#   this single mutex we guarantee that IRC logic executes in one thread at a
#   time.  IRC traffic is mostly idle, so lock contention is negligible.
#
# *+@conns_mutex+* (a +Mutex+):
#   Protects the +@conns+ Hash.  We never hold both mutexes simultaneously,
#   so there is no deadlock risk.
#
# == Writes bypass the handler mutex
#
# +send_to+ looks up the connection under +@conns_mutex+ (briefly), then
# writes *outside* the handler mutex.  This avoids a deadlock where a handler
# callback (holding +@handler_mutex+) tries to write back to the client.

require "socket"
require "thread"

require_relative "irc_net_stdlib/version"

module CodingAdventures
  module IrcNetStdlib
    # Thread-safe monotonically increasing connection ID counter.
    CONN_ID_MUTEX = Mutex.new
    @next_conn_id = 0

    # Allocate a unique connection ID.
    #
    # @return [Integer] a positive integer, unique across the process lifetime
    def self.alloc_conn_id
      CONN_ID_MUTEX.synchronize do
        @next_conn_id += 1
      end
    end

    # Mixin that provides default no-op implementations for Handler callbacks.
    #
    # Include this in any class that wants to be used as a handler without
    # implementing all three callbacks.
    #
    # Example:
    #
    #   class MyHandler
    #     include CodingAdventures::IrcNetStdlib::Handler
    #
    #     def on_data(conn_id, data)
    #       puts "Got #{data.bytesize} bytes from #{conn_id}"
    #     end
    #   end
    module Handler
      # Called when a new TCP connection is accepted.
      # @param conn_id [Integer]
      # @param host    [String]  peer IP address
      def on_connect(conn_id, host); end

      # Called when data arrives from a connection.
      # @param conn_id [Integer]
      # @param data    [String]  raw bytes
      def on_data(conn_id, data); end

      # Called when a connection closes (peer disconnect or error).
      # @param conn_id [Integer]
      def on_disconnect(conn_id); end
    end

    # Wraps a +TCPSocket+ with a thread-safe +write+ method.
    #
    # All reads happen in the connection's dedicated thread.
    # Writes may come from *any* thread (e.g. from a handler responding to
    # a message from another connection), so we protect them with a mutex.
    class StdlibConnection
      def initialize(socket)
        @socket = socket
        @mutex  = Mutex.new
        @closed = false
      end

      # Blocking read.  Returns the received string or +nil+ on close/error.
      #
      # Uses +IO.select+ with a 0.5-second timeout to avoid sleeping forever
      # on a connection that never sends data but hasn't closed.  Loops until
      # data arrives or +@closed+ is set.
      #
      # @return [String, nil]
      def read
        loop do
          return nil if @closed

          ready = IO.select([@socket], nil, nil, 0.5)
          next unless ready

          begin
            data = @socket.recv(4096)
            # recv returns nil or "" on peer close (platform-dependent).
            return nil if data.nil? || data.empty?

            return data
          rescue IOError, Errno::ECONNRESET, Errno::EPIPE
            return nil
          end
        end
      end

      # Write +data+ to the socket.
      #
      # Thread-safe: multiple threads may call +write+ concurrently.
      # Silently ignores errors (the read loop will detect the close).
      #
      # @param data [String]
      def write(data)
        return if @closed

        @mutex.synchronize do
          @socket.write(data)
        rescue IOError, Errno::ECONNRESET, Errno::EPIPE
          @closed = true
        end
      end

      # Close the underlying socket and mark this connection as closed.
      def close
        @closed = true
        @socket.close rescue nil
      end
    end

    # TCP event loop built on Ruby's stdlib +TCPServer+.
    #
    # == Lifecycle
    #
    # 1. +run+ binds a +TCPServer+ and enters an accept loop.
    # 2. For each accepted connection, a daemon thread is spawned to
    #    drive the read loop.
    # 3. +stop+ sets a flag that causes the accept loop to exit after
    #    the next timeout.
    #
    # == Handler protocol
    #
    # The +handler+ object must respond to three methods (see +Handler+):
    #   - +on_connect(conn_id, host)+
    #   - +on_data(conn_id, data)+
    #   - +on_disconnect(conn_id)+
    class StdlibEventLoop
      def initialize
        @conns          = {}   # conn_id → StdlibConnection
        @handler_mutex  = Mutex.new
        @conns_mutex    = Mutex.new
        @running        = false
        @server         = nil
      end

      # Start the event loop.  Blocks until +stop+ is called.
      #
      # @param host    [String]  bind address (e.g. "0.0.0.0")
      # @param port    [Integer] TCP port to listen on
      # @param handler [#on_connect, #on_data, #on_disconnect]
      def run(host, port, handler)
        @running = true
        @server  = TCPServer.new(host, port)
        @server.setsockopt(Socket::SOL_SOCKET, Socket::SO_REUSEADDR, true)

        accept_loop(handler)
      ensure
        @server&.close rescue nil
        @running = false
      end

      # Signal the event loop to stop accepting new connections.
      #
      # Already-established connections are not forcibly closed; their
      # threads will exit when the peer disconnects.
      def stop
        @running = false
        @server&.close rescue nil
      end

      # Send +data+ to the connection identified by +conn_id+.
      #
      # If +conn_id+ is unknown (e.g. the connection already closed), this
      # is a silent no-op.
      #
      # @param conn_id [Integer]
      # @param data    [String]
      # @return [nil]
      def send_to(conn_id, data)
        conn = @conns_mutex.synchronize { @conns[conn_id] }
        conn&.write(data)
        nil
      end

      private

      # Accept loop: wait for connections; spawn a thread per connection.
      def accept_loop(handler)
        loop do
          break unless @running

          # Use IO.select so we can poll @running without blocking forever.
          # On Windows, IO.select raises ENOTSOCK when the server socket is
          # closed — treat that as a clean shutdown signal.
          begin
            ready = IO.select([@server], nil, nil, 0.5)
          rescue IOError, Errno::EBADF, Errno::ENOTSOCK
            break  # server was closed by stop()
          end
          next unless ready

          begin
            socket = @server.accept_nonblock
          rescue IO::WaitReadable
            next
          rescue IOError, Errno::EBADF, Errno::ENOTSOCK
            break  # server was closed by stop()
          end

          spawn_connection_thread(socket, handler)
        end
      end

      # Allocate a +StdlibConnection+, register it, and start its read loop.
      def spawn_connection_thread(socket, handler)
        conn_id = IrcNetStdlib.alloc_conn_id
        host    = socket.peeraddr[3] rescue "unknown"
        conn    = StdlibConnection.new(socket)

        @conns_mutex.synchronize { @conns[conn_id] = conn }

        thread = Thread.new do
          connection_thread_body(conn_id, host, conn, handler)
        end
        thread.abort_on_exception = false
        thread
      end

      # Body of each connection thread.
      #
      # 1. Fire +on_connect+.
      # 2. Loop: read data → fire +on_data+.
      # 3. Fire +on_disconnect+ on close.
      # 4. Remove connection from registry.
      def connection_thread_body(conn_id, host, conn, handler)
        @handler_mutex.synchronize { handler.on_connect(conn_id, host) }

        loop do
          data = conn.read
          break if data.nil?

          @handler_mutex.synchronize { handler.on_data(conn_id, data) }
        end
      ensure
        @handler_mutex.synchronize { handler.on_disconnect(conn_id) }
        @conns_mutex.synchronize   { @conns.delete(conn_id) }
        conn.close
      end
    end
  end
end
