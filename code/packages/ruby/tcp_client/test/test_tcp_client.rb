# frozen_string_literal: true

# = Tests for CodingAdventures::TcpClient
#
# These tests use an echo server on port 0 (OS-assigned) to verify the
# TCP client's behavior without needing external network access. Each
# test spins up a lightweight server in a background thread.
#
# == Test server pattern
#
#   server = TCPServer.new("127.0.0.1", 0)   # OS picks a free port
#   port = server.addr[1]                     # read the assigned port
#   Thread.new { handle_connection(server) }  # serve in background
#
# Using port 0 avoids port conflicts when tests run in parallel. The OS
# guarantees the port is available at the time of binding.

require "minitest/autorun"
require "socket"
require "coding_adventures_tcp_client"

class TestTcpClient < Minitest::Test
  # ======================================================================
  # Helper: start a local echo server on an OS-assigned port
  # ======================================================================
  #
  # The echo server is the simplest useful TCP server:
  #   1. Accept one connection
  #   2. Read data and echo it back byte-for-byte
  #   3. Close when the client closes
  #
  # This lets us test the full round-trip: write -> network -> read.

  def start_echo_server
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]

    thread = Thread.new do
      client = server.accept
      client.binmode
      loop do
        data = begin
          client.readpartial(65_536)
        rescue EOFError
          break
        end
        client.write(data)
        client.flush
      end
      client.close
    rescue IOError
      # Server closed -- expected during cleanup.
      nil
    ensure
      server.close rescue nil # rubocop:disable Style/RescueModifier
    end

    [port, server, thread]
  end

  # ======================================================================
  # Helper: start a server that accepts but never sends data
  # ======================================================================
  #
  # Used for read timeout tests. The server holds the connection open
  # but never writes anything, forcing the client's read to time out.

  def start_silent_server
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]

    thread = Thread.new do
      client = server.accept
      # Hold connection open but never send data
      sleep 30
      client.close
    rescue IOError
      nil
    ensure
      server.close rescue nil # rubocop:disable Style/RescueModifier
    end

    [port, server, thread]
  end

  # ======================================================================
  # Helper: start a server that sends exactly n bytes then closes
  # ======================================================================
  #
  # Used for EOF and partial-read tests. The server writes a fixed
  # payload and then closes the connection.

  def start_partial_server(data)
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]

    thread = Thread.new do
      client = server.accept
      client.binmode
      client.write(data)
      client.flush
      sleep 0.1 # let client read before we close
      client.close
    rescue IOError
      nil
    ensure
      server.close rescue nil # rubocop:disable Style/RescueModifier
    end

    [port, server, thread]
  end

  # ======================================================================
  # Helper: start a request-response server
  # ======================================================================
  #
  # Reads one request from the client, then sends a fixed response.
  # Used to simulate HTTP-like request/response patterns.

  def start_request_response_server(response)
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]

    thread = Thread.new do
      client = server.accept
      client.binmode
      # Read the request (up to 4 KiB)
      begin
        client.readpartial(4096)
      rescue EOFError
        # empty request
      end
      # Send the response
      client.write(response)
      client.flush
      sleep 0.1
      client.close
    rescue IOError
      nil
    ensure
      server.close rescue nil # rubocop:disable Style/RescueModifier
    end

    [port, server, thread]
  end

  # ======================================================================
  # Helper: test-friendly options with short timeouts
  # ======================================================================

  def test_options
    CodingAdventures::TcpClient::ConnectOptions.new(
      connect_timeout: 5,
      read_timeout: 5,
      write_timeout: 5,
      buffer_size: 4096
    )
  end

  # ======================================================================
  # Teardown: clean up server resources
  # ======================================================================

  def teardown
    # Thread cleanup happens automatically via ensure blocks in helpers.
    # This is here as a safety net.
  end

  # ======================================================================
  # Group 1: Basic connectivity
  # ======================================================================

  # Test 1: connect and disconnect
  #
  # The most basic test: can we establish a connection and let it close?
  # If this fails, nothing else will work.
  def test_connect_and_disconnect
    port, _server, thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)
    assert_instance_of CodingAdventures::TcpClient::TcpConnection, conn
    conn.close
    thread.join(2)
  end

  # Test 2: version exists
  #
  # Sanity check that the VERSION constant is defined.
  def test_version_exists
    refute_nil CodingAdventures::TcpClient::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::TcpClient::VERSION)
  end

  # ======================================================================
  # Group 2: Echo server tests (write -> read round-trip)
  # ======================================================================

  # Test 3: write and read back
  #
  # Send bytes, get the same bytes back. This validates the full
  # data path: client write -> network -> server -> network -> client read.
  def test_write_and_read_back
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    conn.write_all("Hello, TCP!")
    conn.flush

    result = conn.read_exact(11)
    assert_equal "Hello, TCP!", result
    conn.close
  end

  # Test 4: read_line from echo server
  #
  # Send two lines, read them back one at a time. Verifies that
  # read_line correctly splits on \n boundaries.
  def test_read_line_from_echo
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    conn.write_all("Hello\r\nWorld\r\n")
    conn.flush

    line1 = conn.read_line
    assert_equal "Hello\r\n", line1

    line2 = conn.read_line
    assert_equal "World\r\n", line2

    conn.close
  end

  # Test 5: read_exact from echo server
  #
  # Send a known byte pattern, read it back with read_exact. Verifies
  # that exactly the right number of bytes are returned.
  def test_read_exact_from_echo
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    data = (0...100).map { |i| (i % 256).chr }.join
    conn.write_all(data)
    conn.flush

    result = conn.read_exact(100)
    assert_equal 100, result.bytesize
    assert_equal data.b, result.b
    conn.close
  end

  # Test 6: read_until from echo server
  #
  # Send data with a null terminator, read until the terminator.
  # Verifies delimiter-based reading for protocols like RESP.
  def test_read_until_from_echo
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    conn.write_all("key:value\0next")
    conn.flush

    result = conn.read_until("\0")
    assert_equal "key:value\0".b, result.b
    conn.close
  end

  # Test 7: large data transfer
  #
  # Send and receive 64 KiB to test buffering and chunked I/O.
  # This exercises the buffer_size boundary -- data must be read
  # in multiple chunks and reassembled correctly.
  def test_large_data_transfer
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    data = (0...65_536).map { |i| (i % 256).chr }.join
    conn.write_all(data)
    conn.flush

    result = conn.read_exact(65_536)
    assert_equal 65_536, result.bytesize
    assert_equal data.b, result.b
    conn.close
  end

  # Test 8: multiple exchanges
  #
  # Two sequential request-response cycles on the same connection.
  # Verifies that the connection state is clean between exchanges.
  def test_multiple_exchanges
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    # Exchange 1
    conn.write_all("ping\n")
    conn.flush
    line1 = conn.read_line
    assert_equal "ping\n", line1

    # Exchange 2
    conn.write_all("pong\n")
    conn.flush
    line2 = conn.read_line
    assert_equal "pong\n", line2

    conn.close
  end

  # ======================================================================
  # Group 3: Timeout tests
  # ======================================================================

  # Test 9: read timeout
  #
  # Connect to a server that never sends data. The read should
  # time out after the configured read_timeout.
  def test_read_timeout
    port, _server, thread = start_silent_server
    opts = CodingAdventures::TcpClient::ConnectOptions.new(
      connect_timeout: 5,
      read_timeout: 1,
      write_timeout: 5,
      buffer_size: 4096
    )
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, opts)

    assert_raises(CodingAdventures::TcpClient::Timeout) do
      conn.read_line
    end

    conn.close
    thread.kill
  end

  # ======================================================================
  # Group 4: Error tests
  # ======================================================================

  # Test 10: connection refused
  #
  # Connect to a port where nothing is listening. The OS immediately
  # responds with TCP RST, causing ECONNREFUSED. On some platforms
  # (notably Windows), this may instead manifest as a timeout, so we
  # accept either error.
  def test_connection_refused
    # Bind a port, then immediately close the server so nothing listens.
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]
    server.close

    assert_raises(
      CodingAdventures::TcpClient::ConnectionRefused,
      CodingAdventures::TcpClient::Timeout
    ) do
      CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)
    end
  end

  # Test 11: DNS resolution failure
  #
  # Try to connect to a hostname that definitely does not exist.
  # The OS resolver should fail with NXDOMAIN.
  def test_dns_failure
    error = assert_raises(CodingAdventures::TcpClient::DnsResolutionFailed) do
      CodingAdventures::TcpClient.connect(
        "this.host.does.not.exist.example",
        80,
        test_options
      )
    end
    assert_includes error.message, "this.host.does.not.exist.example"
  end

  # Test 12: unexpected EOF
  #
  # Server sends 50 bytes then closes. Client tries to read 100.
  # Should raise UnexpectedEof because the connection closed before
  # all requested bytes arrived.
  def test_unexpected_eof
    data = (0...50).map(&:chr).join
    port, _server, _thread = start_partial_server(data)
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    # Wait for server to send data and close
    sleep 0.2

    assert_raises(CodingAdventures::TcpClient::UnexpectedEof) do
      conn.read_exact(100)
    end

    conn.close
  end

  # Test 13: broken pipe (write after server closes)
  #
  # Server accepts then immediately closes. Client tries to write
  # large amounts of data, eventually getting EPIPE.
  def test_broken_pipe
    port, _server, _thread = start_partial_server("")
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    # Wait for server to close its end
    sleep 0.3

    got_error = false
    10.times do
      begin
        conn.write_all("\x00" * 65_536)
        conn.flush
      rescue CodingAdventures::TcpClient::BrokenPipe,
             CodingAdventures::TcpClient::ConnectionReset
        got_error = true
        break
      end
      sleep 0.05
    end

    assert got_error, "expected write error after server closed"
    conn.close
  end

  # ======================================================================
  # Group 5: Half-close tests
  # ======================================================================

  # Test 14: client half-close (shutdown_write)
  #
  # The client sends data, shuts down the write half, then reads the
  # server's response. This tests the half-close mechanism that many
  # protocols rely on.
  #
  #   Timeline:
  #     Client: write("request data")
  #     Client: shutdown_write()        -- signals "I'm done sending"
  #     Server: reads until EOF         -- sees the shutdown
  #     Server: write("DONE\n")         -- sends response
  #     Client: read_line()             -- reads "DONE\n"
  def test_client_half_close
    server = TCPServer.new("127.0.0.1", 0)
    port = server.addr[1]

    server_received = +"" # mutex not needed: only server thread writes
    thread = Thread.new do
      client = server.accept
      client.binmode
      # Read until EOF (client shuts down write half)
      buf = +""
      loop do
        chunk = begin
          client.readpartial(1024)
        rescue EOFError
          break
        end
        buf << chunk
      end
      server_received.replace(buf)
      # Send response
      client.write("DONE\n")
      client.flush
      sleep 0.1
      client.close
    ensure
      server.close rescue nil # rubocop:disable Style/RescueModifier
    end

    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    conn.write_all("request data")
    conn.shutdown_write

    # Read the server's response
    response = conn.read_line
    assert_equal "DONE\n", response

    thread.join(5)
    assert_equal "request data", server_received
    conn.close
  end

  # ======================================================================
  # Group 6: Edge cases
  # ======================================================================

  # Test 15: empty read at EOF
  #
  # After the server sends data and closes, read_line returns the
  # data first, then returns "" to signal EOF.
  def test_empty_read_at_eof
    port, _server, _thread = start_partial_server("hello\n")
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    sleep 0.2

    line = conn.read_line
    assert_equal "hello\n", line

    # Next read should return empty string (EOF)
    eof_line = conn.read_line
    assert_equal "", eof_line

    conn.close
  end

  # Test 16: zero byte write
  #
  # Writing zero bytes should succeed without error. This is a
  # degenerate case that should be a no-op.
  def test_zero_byte_write
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    # Should not raise
    conn.write_all("")
    conn.close
  end

  # Test 17: peer and local addresses
  #
  # Verify that peer_addr returns the server's address and local_addr
  # returns a valid local address with an ephemeral port.
  def test_peer_and_local_address
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    host, peer_port = conn.peer_addr
    assert_equal "127.0.0.1", host
    assert_equal port, peer_port

    local_host, local_port = conn.local_addr
    assert_equal "127.0.0.1", local_host
    assert_operator local_port, :>, 0

    conn.close
  end

  # Test 18: connect_options defaults
  #
  # Verify that default options match the documented values.
  def test_connect_options_defaults
    opts = CodingAdventures::TcpClient::ConnectOptions.new

    assert_equal 30, opts.connect_timeout
    assert_equal 30, opts.read_timeout
    assert_equal 30, opts.write_timeout
    assert_equal 8192, opts.buffer_size
  end

  # Test 19: connect with nil options uses defaults
  #
  # Passing nil for options should use the defaults (30s timeouts, 8 KiB buffer).
  def test_connect_with_nil_options
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port)
    assert_instance_of CodingAdventures::TcpClient::TcpConnection, conn
    conn.close
  end

  # Test 20: request-response pattern (HTTP-like)
  #
  # Simulates an HTTP/1.0 exchange: send a request, read status line,
  # headers, blank line, and body. This is the primary use case for
  # the TCP client.
  def test_request_response_pattern
    response_data = "HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello"
    port, _server, _thread = start_request_response_server(response_data)

    conn = CodingAdventures::TcpClient.connect("127.0.0.1", port, test_options)

    # Send request
    conn.write_all("GET / HTTP/1.0\r\n\r\n")
    conn.flush

    # Read response line by line
    status = conn.read_line
    assert status.start_with?("HTTP/1.0 200"), "expected HTTP 200, got: #{status}"

    header = conn.read_line
    assert header.start_with?("Content-Length:"), "expected Content-Length header"

    blank = conn.read_line
    assert_equal "\r\n", blank

    body = conn.read_exact(5)
    assert_equal "hello", body

    conn.close
  end

  # ======================================================================
  # Group 7: Error hierarchy tests
  # ======================================================================

  # Test 21 (bonus): error hierarchy
  #
  # All specific errors should be subclasses of TcpError, which itself
  # is a StandardError. This lets callers rescue TcpError to catch any
  # TCP-related error.
  def test_error_hierarchy
    assert CodingAdventures::TcpClient::DnsResolutionFailed < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::ConnectionRefused < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::Timeout < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::ConnectionReset < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::BrokenPipe < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::UnexpectedEof < CodingAdventures::TcpClient::TcpError
    assert CodingAdventures::TcpClient::TcpError < StandardError
  end

  # Test 22 (bonus): connect with localhost hostname
  #
  # "localhost" should resolve to 127.0.0.1 via the OS resolver.
  def test_connect_with_hostname_localhost
    port, _server, _thread = start_echo_server
    conn = CodingAdventures::TcpClient.connect("localhost", port, test_options)
    assert_instance_of CodingAdventures::TcpClient::TcpConnection, conn
    conn.close
  end
end
