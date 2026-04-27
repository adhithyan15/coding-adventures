# frozen_string_literal: true

# = Error hierarchy for CodingAdventures::TcpClient
#
# Each error maps to a specific failure mode during TCP communication.
# Match on these to decide how to recover:
#
#   | Error class          | Meaning                                         |
#   |----------------------|-------------------------------------------------|
#   | DnsResolutionFailed  | hostname typo or no internet                    |
#   | ConnectionRefused    | server up but nothing listening on that port     |
#   | Timeout              | took too long (connect, read, or write)          |
#   | ConnectionReset      | remote side crashed (TCP RST)                   |
#   | BrokenPipe           | tried to write after remote closed               |
#   | UnexpectedEof        | connection closed before expected data arrived   |
#
# All errors inherit from +TcpError+, which itself inherits from
# +StandardError+. This means you can rescue +TcpError+ to catch
# any TCP-related error, or rescue a specific subclass for fine-grained
# handling.
#
# == Example
#
#   begin
#     conn = CodingAdventures::TcpClient.connect("example.com", 80)
#   rescue CodingAdventures::TcpClient::DnsResolutionFailed
#     puts "Check your hostname"
#   rescue CodingAdventures::TcpClient::ConnectionRefused
#     puts "Server is not accepting connections"
#   rescue CodingAdventures::TcpClient::TcpError => e
#     puts "Some other TCP error: #{e.message}"
#   end

module CodingAdventures
  module TcpClient
    # Base error class for all TCP client errors.
    # Rescue this to catch any TCP-related error.
    class TcpError < StandardError; end

    # DNS lookup failed -- hostname could not be resolved.
    #
    # Common causes:
    # - Typo in the hostname ("exmaple.com")
    # - No internet connection
    # - DNS server is down
    class DnsResolutionFailed < TcpError; end

    # Server is reachable but nothing is listening on the port.
    #
    # The remote OS responded with a TCP RST packet during the handshake.
    # This means the machine exists, but no process is bound to that port.
    class ConnectionRefused < TcpError; end

    # Operation timed out (connect, read, or write).
    #
    # Without timeouts, a stalled server could hang your program forever.
    # The three timeout phases:
    #
    #   connect_timeout -- waiting for TCP handshake to complete
    #   read_timeout    -- waiting for data after calling read
    #   write_timeout   -- waiting for OS send buffer to drain
    class Timeout < TcpError; end

    # Remote side reset the connection unexpectedly (TCP RST during transfer).
    #
    # This usually means the server process crashed or was killed while
    # the connection was active.
    class ConnectionReset < TcpError; end

    # Tried to write to a connection the remote side already closed.
    #
    # The remote end has closed its read side, so our writes have nowhere
    # to go. The OS delivers a SIGPIPE / EPIPE error.
    class BrokenPipe < TcpError; end

    # Connection closed before the expected number of bytes arrived.
    #
    # For example, you called read_exact(100) but only 50 bytes were
    # available before the connection closed.
    class UnexpectedEof < TcpError; end
  end
end
