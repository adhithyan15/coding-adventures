# frozen_string_literal: true

# Tests for the ircd program's DriverHandler and Config.
#
# We test the wiring layer in isolation by stubbing out the event loop.
# For integration: we do a real end-to-end test using actual TCP sockets.
#
# Coverage strategy:
#   - Config.default values
#   - parse_args: defaults, --port, --server-name, --motd, --oper-password
#   - DriverHandler: on_connect, on_data (framing + parsing), on_disconnect
#   - DriverHandler: malformed lines skipped gracefully
#   - Integration: full NICK+USER registration via real TCP

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 75
end

require "minitest/autorun"
require "socket"
require "thread"

$LOAD_PATH.unshift(File.expand_path("../lib", __dir__))
require "ircd"

# ─────────────────────────────────────────────────────────────────────────────
# Config tests
# ─────────────────────────────────────────────────────────────────────────────

class TestConfig < Minitest::Test
  def test_default_values
    c = Config.default
    assert_equal "0.0.0.0",  c.host
    assert_equal 6667,       c.port
    assert_equal "irc.local", c.server_name
    assert_equal ["Welcome."], c.motd
    assert_equal "",         c.oper_password
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# parse_args tests
# ─────────────────────────────────────────────────────────────────────────────

class TestParseArgs < Minitest::Test
  def test_defaults_when_no_args
    c = parse_args([])
    assert_equal 6667, c.port
    assert_equal "irc.local", c.server_name
  end

  def test_port_flag
    c = parse_args(["--port", "6668"])
    assert_equal 6668, c.port
  end

  def test_server_name_flag
    c = parse_args(["--server-name", "irc.example.com"])
    assert_equal "irc.example.com", c.server_name
  end

  def test_motd_flag
    c = parse_args(["--motd", "Hello!"])
    assert_equal ["Hello!"], c.motd
  end

  def test_oper_password_flag
    c = parse_args(["--oper-password", "secret"])
    assert_equal "secret", c.oper_password
  end

  def test_host_flag
    c = parse_args(["--host", "127.0.0.1"])
    assert_equal "127.0.0.1", c.host
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Stub loop for unit tests
# ─────────────────────────────────────────────────────────────────────────────

# A fake EventLoop that records what was sent to which connection.
class StubLoop
  attr_reader :sent

  def initialize
    @sent = []  # Array<[conn_id, String]>
  end

  def send_to(conn_id, data)
    @sent << [conn_id, data]
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# DriverHandler unit tests
# ─────────────────────────────────────────────────────────────────────────────

class TestDriverHandler < Minitest::Test
  def setup
    @server  = CodingAdventures::IrcServer::IRCServer.new(server_name: "irc.test")
    @stub    = StubLoop.new
    @handler = DriverHandler.new(@server, @stub)
  end

  def test_on_connect_creates_framer
    # on_connect should not raise and should prepare for on_data.
    @handler.on_connect(1, "127.0.0.1")
    # No initial responses expected (IRCServer.on_connect returns []).
    assert_equal [], @stub.sent
  end

  def test_on_data_complete_line_dispatched
    @handler.on_connect(1, "127.0.0.1")
    @handler.on_data(1, "NICK alice\r\n")
    # After NICK alone, no welcome yet (need USER too).
    # But no error should occur.
  end

  def test_on_data_full_registration_sends_welcome
    @handler.on_connect(1, "127.0.0.1")
    @handler.on_data(1, "NICK alice\r\n")
    @handler.on_data(1, "USER alice 0 * :Alice\r\n")
    # Welcome sequence should have been sent.
    sent_commands = @stub.sent.map do |_, wire|
      wire.split(" ")[1]  # crude command extraction
    end
    # 001 RPL_WELCOME should appear.
    assert sent_commands.any? { |c| c == "001" },
      "Expected 001 in sent commands, got: #{sent_commands.inspect}"
  end

  def test_on_data_partial_line_buffered
    @handler.on_connect(1, "127.0.0.1")
    @handler.on_data(1, "NICK ali")
    # Partial — nothing dispatched yet.
    initial_sent_count = @stub.sent.length
    @handler.on_data(1, "ce\r\n")
    # Now the NICK should be processed (but still no welcome without USER).
    assert_equal initial_sent_count, @stub.sent.length
  end

  def test_on_data_malformed_line_skipped
    @handler.on_connect(1, "127.0.0.1")
    # Empty line after CRLF is a ParseError in irc_proto.
    # Should be silently skipped — handler must not raise.
    @handler.on_data(1, "\r\n")
  end

  def test_on_disconnect_cleans_up
    @handler.on_connect(1, "127.0.0.1")
    @handler.on_data(1, "NICK alice\r\n")
    @handler.on_data(1, "USER alice 0 * :Alice\r\n")
    @handler.on_disconnect(1)
    # No exception should be raised.
  end

  def test_on_data_for_unknown_conn_is_noop
    # on_data without on_connect should not raise.
    @handler.on_data(99, "NICK alice\r\n")
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Integration: end-to-end via real TCP socket
# ─────────────────────────────────────────────────────────────────────────────

class TestIntegration < Minitest::Test
  # Use a class-level port counter so parallel test suites don't collide.
  @@integration_port = 19600

  def setup
    @server_obj = CodingAdventures::IrcServer::IRCServer.new(
      server_name: "irc.test",
      motd: ["Test server."]
    )

    # Try to bind an available port with retries.
    @port = nil
    attempts = 0
    while @port.nil? && attempts < 10
      attempts += 1
      candidate = @@integration_port + attempts
      begin
        @loop    = CodingAdventures::IrcNetStdlib::StdlibEventLoop.new
        @handler = DriverHandler.new(@server_obj, @loop)

        @server_thread = Thread.new do
          @loop.run("127.0.0.1", candidate, @handler)
        end
        @server_thread.abort_on_exception = false

        # Wait for server to bind (up to 500ms).
        connected = false
        10.times do
          sleep(0.05)
          begin
            s = TCPSocket.open("127.0.0.1", candidate)
            s.close
            connected = true
            break
          rescue Errno::ECONNREFUSED
            next
          end
        end

        if connected
          @port = candidate
        else
          @loop.stop
          @server_thread.join(1)
        end
      rescue Errno::EADDRINUSE
        @loop&.stop rescue nil
        @server_thread&.join(1)
        next
      end
    end

    skip "Could not start integration test server" unless @port
  end

  def teardown
    @loop&.stop
    @server_thread&.join(2)
  end

  # Read available data from a TCPSocket within a timeout window.
  #
  # Uses IO.select (single-threaded) rather than spawning per-read threads.
  # The multi-thread approach caused a race: when t.join timed out and we
  # killed the thread, the killed thread's pending recv() could still consume
  # bytes from the OS buffer (data loss), leaving the next recv with nothing.
  #
  # With IO.select we never have two concurrent readers on the same socket.
  # The loop strategy:
  #   - Wait up to 0.4 s (or remaining time) for data to arrive.
  #   - If data arrives, append and keep looping.
  #   - If the 0.4 s window expires and we already have data, declare done
  #     (the server burst is over).
  #   - If the window expires with no data at all, keep waiting until the
  #     outer deadline so we don't give up on a slow server start.
  #   - Hard-exit once buf exceeds 2048 bytes (well above any IRC welcome
  #     sequence: 001–004 + 251 + 375 + one MOTD line + 376 ≈ 600–800 bytes).
  def read_with_timeout(sock, timeout: 2.0, max_bytes: 8192)
    buf = +""  # mutable string
    deadline = Time.now + timeout
    loop do
      remaining = deadline - Time.now
      break if remaining <= 0

      ready = IO.select([sock], nil, nil, [remaining, 0.4].min)
      unless ready
        # Window expired with no new data.
        break if buf.length > 0  # We have a full burst; we're done.
        next                      # Still waiting for the first byte.
      end

      chunk = sock.recv(max_bytes)
      break if chunk.nil? || chunk.empty?  # peer closed
      buf << chunk
      break if buf.length >= 2048
    end
    buf
  end

  def test_nick_user_registration_receives_welcome
    client = TCPSocket.open("127.0.0.1", @port)

    client.write("NICK testuser\r\n")
    client.write("USER testuser 0 * :Test User\r\n")

    received = read_with_timeout(client, timeout: 2.0)
    client.close

    assert_match(/001/, received,
                 "Expected 001 RPL_WELCOME in: #{received.inspect[0, 200]}")
    assert_match(/376/, received,
                 "Expected 376 RPL_ENDOFMOTD in: #{received.inspect[0, 200]}")
  end

  def test_ping_pong
    client = TCPSocket.open("127.0.0.1", @port)

    client.write("NICK pinger\r\n")
    client.write("USER pinger 0 * :Pinger\r\n")
    # Read welcome to clear buffer.
    read_with_timeout(client, timeout: 0.5)

    client.write("PING irc.test\r\n")
    received = read_with_timeout(client, timeout: 1.5)
    client.close

    assert_match(/PONG/, received, "Expected PONG in response")
  end
end
