# frozen_string_literal: true

# = ConnectOptions -- configuration for establishing a TCP connection
#
# All timeouts default to 30 seconds. The buffer size defaults to 8192
# bytes (8 KiB), a good balance between memory usage and syscall reduction.
#
# == Why separate timeouts?
#
#   connect_timeout (30s) -- how long to wait for the TCP handshake
#     If a server is down or firewalled, the OS might wait minutes.
#
#   read_timeout (30s) -- how long to wait for data after calling read
#     Without this, a stalled server hangs your program forever.
#
#   write_timeout (30s) -- how long to wait for the OS send buffer
#     Usually instant, but blocks if the remote side isn't reading.
#
#   buffer_size (8192) -- internal read buffer size in bytes
#     Larger buffers mean fewer syscalls but more memory per connection.
#     8 KiB is Ruby's default IO buffer size -- a battle-tested default.
#
# == Example
#
#   # Use all defaults
#   opts = CodingAdventures::TcpClient::ConnectOptions.new
#
#   # Custom timeouts for a slow server
#   opts = CodingAdventures::TcpClient::ConnectOptions.new(
#     connect_timeout: 60,
#     read_timeout: 120
#   )
#
#   # Tiny buffer for memory-constrained environments
#   opts = CodingAdventures::TcpClient::ConnectOptions.new(buffer_size: 1024)

module CodingAdventures
  module TcpClient
    class ConnectOptions
      # Maximum time in seconds to wait for the TCP handshake. Default: 30.
      attr_accessor :connect_timeout

      # Maximum time in seconds to wait for data on read. Default: 30.
      attr_accessor :read_timeout

      # Maximum time in seconds to wait on write. Default: 30.
      attr_accessor :write_timeout

      # Size of the internal read buffer in bytes. Default: 8192.
      attr_accessor :buffer_size

      # Default timeout values in seconds.
      #
      #   | Setting         | Default | Why                                    |
      #   |-----------------|---------|----------------------------------------|
      #   | connect_timeout | 30      | Most servers respond within seconds    |
      #   | read_timeout    | 30      | Prevents hanging on stalled servers    |
      #   | write_timeout   | 30      | Prevents hanging on full send buffers  |
      #   | buffer_size     | 8192    | Matches Ruby's default IO buffer size  |
      DEFAULT_CONNECT_TIMEOUT = 30
      DEFAULT_READ_TIMEOUT = 30
      DEFAULT_WRITE_TIMEOUT = 30
      DEFAULT_BUFFER_SIZE = 8192

      def initialize(
        connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        read_timeout: DEFAULT_READ_TIMEOUT,
        write_timeout: DEFAULT_WRITE_TIMEOUT,
        buffer_size: DEFAULT_BUFFER_SIZE
      )
        @connect_timeout = connect_timeout
        @read_timeout = read_timeout
        @write_timeout = write_timeout
        @buffer_size = buffer_size
      end
    end
  end
end
