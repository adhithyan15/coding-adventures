# frozen_string_literal: true

# = CodingAdventures::TcpClient
#
# A TCP client with buffered I/O and configurable timeouts.
#
# This module wraps Ruby's +TCPSocket+ with ergonomic defaults for building
# network clients. It is *protocol-agnostic* -- it knows nothing about HTTP,
# SMTP, or Redis. It just moves bytes reliably between two machines. Higher-
# level packages build application protocols on top.
#
# == Analogy: A telephone call
#
#   Making a TCP connection is like making a phone call:
#
#   1. DIAL (DNS + connect)
#      Look up "Grandma" -> 555-0123     (DNS resolution)
#      Dial and wait for ring            (TCP three-way handshake)
#      If nobody picks up -> hang up      (connect timeout)
#
#   2. TALK (read/write)
#      Say "Hello, Grandma!"             (write_all + flush)
#      Listen for response               (read_line)
#      If silence for 30s -> "Still there?" (read timeout)
#
#   3. HANG UP (shutdown/close)
#      Say "Goodbye" and hang up         (shutdown_write + close)
#
# == Where it fits
#
#   url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
#                            |
#                       raw byte stream
#
# == Example
#
#   conn = CodingAdventures::TcpClient.connect("info.cern.ch", 80)
#   conn.write_all("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n")
#   conn.flush
#   status_line = conn.read_line
#   puts status_line
#   conn.close

require "socket"

require_relative "coding_adventures/tcp_client/version"
require_relative "coding_adventures/tcp_client/errors"
require_relative "coding_adventures/tcp_client/connect_options"
require_relative "coding_adventures/tcp_client/tcp_connection"

module CodingAdventures
  module TcpClient
    # ========================================================================
    # connect() -- establish a TCP connection
    # ========================================================================

    # Establish a TCP connection to the given host and port.
    #
    # == Algorithm
    #
    #   1. DNS resolution: (host, port) -> [addr1, addr2, ...]
    #      Uses the OS resolver (respects /etc/hosts, system DNS).
    #
    #   2. Try connecting with Socket.tcp which handles:
    #      - Resolving the hostname to one or more addresses
    #      - Trying each address in order (Happy Eyeballs style)
    #      - Applying the connect timeout
    #
    #   3. Configure the connected socket:
    #      Set read and write timeouts via IO.select wrappers.
    #
    #   4. Wrap in TcpConnection with the options.
    #
    # == Error mapping
    #
    #   Ruby's socket library raises various exceptions. We catch them and
    #   translate to our structured error hierarchy:
    #
    #   | Ruby exception       | Our error              |
    #   |----------------------|------------------------|
    #   | SocketError          | DnsResolutionFailed    |
    #   | Errno::ECONNREFUSED  | ConnectionRefused      |
    #   | Errno::ETIMEDOUT     | Timeout                |
    #   | Errno::EHOSTUNREACH  | Timeout                |
    #   | Errno::ENETUNREACH   | Timeout                |
    #
    # == Example
    #
    #   # With defaults (30s timeouts, 8192 buffer)
    #   conn = CodingAdventures::TcpClient.connect("example.com", 80)
    #
    #   # With custom options
    #   opts = CodingAdventures::TcpClient::ConnectOptions.new(
    #     connect_timeout: 10,
    #     read_timeout: 5
    #   )
    #   conn = CodingAdventures::TcpClient.connect("example.com", 80, opts)
    #
    def self.connect(host, port, options = nil)
      options ||= ConnectOptions.new

      # Step 1 & 2: DNS resolution + connect with timeout
      #
      # Socket.tcp handles both DNS resolution and connection. It resolves
      # the hostname, tries each address, and applies the connect timeout.
      # The connect_timeout: keyword argument controls how long to wait for
      # the TCP three-way handshake to complete.
      socket = Socket.tcp(host, port, connect_timeout: options.connect_timeout)

      # Step 3: Return a TcpConnection wrapping the connected socket
      TcpConnection.new(socket, host, port, options)
    rescue SocketError => e
      # DNS resolution failed -- hostname could not be resolved.
      #
      # SocketError is raised when getaddrinfo() fails, meaning the OS
      # resolver could not translate the hostname to an IP address.
      raise DnsResolutionFailed, "DNS resolution failed for '#{host}': #{e.message}"
    rescue Errno::ECONNREFUSED
      # Server is reachable but nothing is listening on the port.
      #
      # The remote OS responded with a TCP RST packet, meaning the port
      # is closed. This is fast -- typically returns within milliseconds.
      raise ConnectionRefused, "connection refused by #{host}:#{port}"
    rescue Errno::ETIMEDOUT
      # Connection attempt timed out at the OS level.
      raise Timeout, "connect timed out after #{options.connect_timeout}s"
    rescue Errno::EHOSTUNREACH, Errno::ENETUNREACH
      # Host or network unreachable -- treat as a timeout scenario.
      raise Timeout, "host unreachable: #{host}:#{port}"
    rescue Errno::EADDRNOTAVAIL
      # The address is not available (e.g., binding to a non-local address).
      raise ConnectionRefused, "address not available: #{host}:#{port}"
    end
  end
end
