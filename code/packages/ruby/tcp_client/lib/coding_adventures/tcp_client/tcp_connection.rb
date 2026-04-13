# frozen_string_literal: true

# = TcpConnection -- buffered I/O over a TCP stream
#
# Wraps a +Socket+ (or +TCPSocket+) with timeout-aware read and write
# operations. All reads use +IO.select+ to enforce the configured timeout
# before delegating to the underlying socket.
#
# == Why buffered I/O?
#
#   Without buffering:
#     read() returns arbitrary chunks: "HT", "TP/", "1.0 2", "00 OK\r\n"
#     100 read() calls = 100 syscalls (expensive!)
#
#   With an internal buffer:
#     First read() pulls a chunk from the OS into memory
#     Subsequent read_line() calls serve data from the buffer
#     100 lines might need only 1-2 syscalls
#
#   Similarly, we flush writes explicitly so small writes are batched
#   into larger TCP segments rather than sending tiny packets.
#
# == Timeout enforcement
#
# Ruby's +TCPSocket+ does not natively support per-operation timeouts.
# We use +IO.select([socket], nil, nil, timeout)+ before each read to
# check if data is available. If +IO.select+ returns +nil+, it means
# the timeout expired and we raise +Timeout+.
#
#   IO.select behavior:
#     Returns [[socket], [], []]  -- data available, safe to read
#     Returns nil                  -- timeout expired, no data
#
# For writes, we use +IO.select(nil, [socket], nil, timeout)+ to check
# if the socket is writable (i.e., the OS send buffer has space).
#
# == The connection is closed when +close+ is called or when the
# object is garbage collected (Ruby closes the socket in its finalizer).

module CodingAdventures
  module TcpClient
    class TcpConnection
      # Create a new TcpConnection wrapping an already-connected socket.
      #
      # This is called by +TcpClient.connect+ -- you typically don't call
      # it directly.
      #
      # == Parameters
      #
      # +socket+  -- a connected Socket or TCPSocket
      # +host+    -- the remote hostname (for address reporting)
      # +port+    -- the remote port number (for address reporting)
      # +options+ -- a ConnectOptions with timeout and buffer settings
      def initialize(socket, host, port, options)
        @socket = socket
        @host = host
        @port = port
        @options = options

        # We use a read buffer to support line-oriented and delimiter-based
        # reading. Data is read from the socket in chunks (up to buffer_size
        # bytes) and stored here. Subsequent read operations consume from
        # this buffer first before going back to the socket.
        @read_buffer = String.new(encoding: Encoding::BINARY)

        # Track whether the socket has been closed to prevent double-close
        # errors and provide clear error messages.
        @closed = false
      end

      # ====================================================================
      # Read operations
      # ====================================================================

      # Read bytes until a newline (+\n+) is found.
      #
      # Returns the line *including* the trailing +\n+ (and +\r\n+ if
      # present). Returns an empty string at EOF (remote closed cleanly).
      #
      # This is the workhorse for line-oriented protocols like HTTP/1.0,
      # SMTP, and RESP (Redis protocol).
      #
      # == Algorithm
      #
      #   1. Check the read buffer for a newline
      #   2. If found, extract and return everything up to and including it
      #   3. If not found, read more data from the socket into the buffer
      #   4. Repeat until newline found or EOF
      #
      # == Example
      #
      #   line = conn.read_line
      #   # => "HTTP/1.0 200 OK\r\n"
      #
      # == Errors
      #
      # - +Timeout+ if no data arrives within the read timeout
      # - +ConnectionReset+ if the remote side closed unexpectedly
      def read_line
        loop do
          # Check if we already have a complete line in the buffer.
          # String#index is O(n) but our buffer is typically small (< 8 KiB).
          newline_pos = @read_buffer.index("\n")

          if newline_pos
            # Found a newline -- extract everything up to and including it.
            # The +1 accounts for the newline character itself.
            line = @read_buffer.slice!(0..newline_pos)
            return line.force_encoding(Encoding::UTF_8)
          end

          # No newline yet -- read more data from the socket.
          chunk = read_chunk
          return "" if chunk.nil? || chunk.empty? # EOF

          @read_buffer << chunk
        end
      end

      # Read exactly +n+ bytes from the connection.
      #
      # Blocks until all +n+ bytes have been received. Useful for protocols
      # that specify an exact content length (e.g., HTTP Content-Length).
      #
      # == Algorithm
      #
      #   1. While buffer has fewer than n bytes, read more from socket
      #   2. If EOF before n bytes, raise UnexpectedEof
      #   3. Extract exactly n bytes from the buffer
      #
      # == Example
      #
      #   body = conn.read_exact(content_length)
      #   # => "Hello, world!" (exactly content_length bytes)
      #
      # == Errors
      #
      # - +UnexpectedEof+ if the connection closes before +n+ bytes arrive
      # - +Timeout+ if no data arrives within the read timeout
      def read_exact(n)
        # Keep reading until we have enough bytes in the buffer.
        while @read_buffer.bytesize < n
          chunk = read_chunk

          if chunk.nil? || chunk.empty?
            # EOF before we got enough bytes.
            received = @read_buffer.bytesize
            raise UnexpectedEof,
                  "unexpected EOF: expected #{n} bytes, got #{received}"
          end

          @read_buffer << chunk
        end

        # Extract exactly n bytes from the front of the buffer.
        data = @read_buffer.slice!(0, n)
        data
      end

      # Read bytes until the given delimiter byte is found.
      #
      # Returns all bytes up to *and including* the delimiter. Useful for
      # protocols with custom delimiters (RESP uses +\r\n+, null-terminated
      # strings use +\0+).
      #
      # == Parameters
      #
      # +delimiter+ -- a single-character String (e.g., "\0", "\n")
      #
      # == Algorithm
      #
      #   1. Search buffer for delimiter byte
      #   2. If found, extract and return everything up to and including it
      #   3. If not found, read more from socket
      #   4. Repeat until found or EOF
      #
      # == Example
      #
      #   data = conn.read_until("\0")
      #   # => "key:value\0"
      def read_until(delimiter)
        loop do
          delim_pos = @read_buffer.index(delimiter)

          if delim_pos
            # Found the delimiter -- extract everything up to and including it.
            end_pos = delim_pos + delimiter.bytesize - 1
            data = @read_buffer.slice!(0..end_pos)
            return data
          end

          chunk = read_chunk
          return @read_buffer.slice!(0, @read_buffer.bytesize) if chunk.nil? || chunk.empty?

          @read_buffer << chunk
        end
      end

      # ====================================================================
      # Write operations
      # ====================================================================

      # Write all bytes to the connection.
      #
      # Data is sent directly through the socket. For small writes, you
      # should batch them and call +flush+ after a complete message.
      #
      # == Parameters
      #
      # +data+ -- a String of bytes to send
      #
      # == Errors
      #
      # - +BrokenPipe+ if the remote side has closed the connection
      # - +Timeout+ if the OS send buffer is full for too long
      def write_all(data)
        data = data.b if data.encoding != Encoding::BINARY
        remaining = data
        while remaining.bytesize > 0
          wait_writable
          begin
            written = @socket.write_nonblock(remaining, exception: false)

            if written == :wait_writable
              # Should not happen after IO.select, but handle gracefully.
              next
            end

            remaining = remaining.byteslice(written, remaining.bytesize - written)
          rescue Errno::ECONNRESET, Errno::ECONNABORTED
            raise ConnectionReset, "connection reset by peer"
          rescue Errno::EPIPE
            raise BrokenPipe, "broken pipe (remote closed)"
          rescue IOError => e
            raise BrokenPipe, "broken pipe: #{e.message}"
          end
        end
      end

      # Flush the write buffer, sending all buffered data to the network.
      #
      # Ruby's +Socket+ does not buffer writes the way +BufWriter+ does in
      # Rust, so this is mostly a no-op for sockets. However, calling flush
      # ensures any OS-level buffering (Nagle's algorithm) is flushed, and
      # maintains API parity with the Rust implementation.
      def flush
        @socket.flush
      rescue Errno::ECONNRESET, Errno::ECONNABORTED
        raise ConnectionReset, "connection reset by peer"
      rescue Errno::EPIPE
        raise BrokenPipe, "broken pipe (remote closed)"
      end

      # ====================================================================
      # Connection management
      # ====================================================================

      # Shut down the write half of the connection (half-close).
      #
      # Signals to the remote side that no more data will be sent. The
      # read half remains open -- you can still receive data.
      #
      #   Before shutdown_write():
      #     Client <-> Server  (full-duplex, both directions open)
      #
      #   After shutdown_write():
      #     Client <- Server   (client can still READ)
      #     Client X Server    (client can no longer WRITE)
      #
      # This is essential for protocols where the server waits for the
      # client to finish sending before responding (e.g., HTTP request
      # body followed by a response).
      #
      # == Example
      #
      #   conn.write_all("request data")
      #   conn.shutdown_write          # tell server we're done
      #   response = conn.read_line    # read server's response
      def shutdown_write
        @socket.shutdown(:WR)
      rescue Errno::ENOTCONN
        # Already disconnected -- shutting down is a no-op.
        nil
      end

      # Returns the remote address as a +[host, port]+ pair.
      #
      # == Example
      #
      #   host, port = conn.peer_addr
      #   # => ["93.184.216.34", 80]
      def peer_addr
        addr = @socket.remote_address
        [addr.ip_address, addr.ip_port]
      end

      # Returns the local address as a +[host, port]+ pair.
      #
      # The local port is assigned by the OS when the connection is
      # established. It's an ephemeral port (typically 49152-65535).
      #
      # == Example
      #
      #   host, port = conn.local_addr
      #   # => ["192.168.1.100", 54321]
      def local_addr
        addr = @socket.local_address
        [addr.ip_address, addr.ip_port]
      end

      # Close the connection, releasing the underlying socket.
      #
      # After calling close, any further read or write operations will
      # raise an error. It's safe to call close multiple times -- subsequent
      # calls are no-ops.
      def close
        return if @closed

        @closed = true
        @socket.close
      rescue IOError
        # Already closed -- ignore.
        nil
      end

      private

      # ====================================================================
      # Internal: read a chunk of data from the socket with timeout
      # ====================================================================

      # Read up to +buffer_size+ bytes from the socket, enforcing the
      # configured read timeout.
      #
      # == Algorithm
      #
      #   1. Call IO.select to wait for data with timeout
      #   2. If timeout expires (IO.select returns nil), raise Timeout
      #   3. If data available, read with read_nonblock
      #   4. If EOF (read_nonblock returns nil or raises EOFError), return nil
      #
      # == Why IO.select + read_nonblock?
      #
      # Ruby's blocking +read+ does not support timeouts. The standard
      # pattern is:
      #
      #   1. IO.select([socket], nil, nil, timeout) -- wait with timeout
      #   2. socket.read_nonblock(n) -- read without blocking
      #
      # This gives us precise timeout control without threads or signals.
      def read_chunk
        wait_readable

        begin
          data = @socket.read_nonblock(@options.buffer_size, exception: false)

          case data
          when :wait_readable
            # Spurious wakeup from IO.select -- try again.
            # This can happen on some platforms when a signal is delivered.
            read_chunk
          when nil
            # EOF -- remote closed the connection cleanly.
            nil
          else
            data.force_encoding(Encoding::BINARY)
          end
        rescue EOFError
          nil
        rescue Errno::ECONNRESET, Errno::ECONNABORTED
          raise ConnectionReset, "connection reset by peer"
        end
      end

      # Wait until the socket is readable, or raise Timeout.
      #
      # IO.select returns:
      #   [[socket], [], []]  -- data available, safe to read
      #   nil                  -- timeout expired, no data
      def wait_readable
        return if @options.read_timeout.nil?

        result = IO.select([@socket], nil, nil, @options.read_timeout)
        return unless result.nil?

        raise Timeout, "read timed out after #{@options.read_timeout}s"
      end

      # Wait until the socket is writable, or raise Timeout.
      #
      # IO.select returns:
      #   [[], [socket], []]  -- buffer has space, safe to write
      #   nil                  -- timeout expired
      def wait_writable
        return if @options.write_timeout.nil?

        result = IO.select(nil, [@socket], nil, @options.write_timeout)
        return unless result.nil?

        raise Timeout, "write timed out after #{@options.write_timeout}s"
      end
    end
  end
end
