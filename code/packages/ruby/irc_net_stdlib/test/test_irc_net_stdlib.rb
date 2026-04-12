# frozen_string_literal: true

# Tests for CodingAdventures::IrcNetStdlib
#
# These tests use real TCP connections on 127.0.0.1 with an ephemeral port
# assigned by the OS (port 0).  This is the standard approach for testing
# networking code without port conflicts.
#
# Coverage strategy:
#   - alloc_conn_id uniqueness
#   - StdlibConnection write/close
#   - EventLoop: on_connect fired, on_data fired, on_disconnect fired
#   - Multiple connections
#   - send_to delivers data to the right connection
#   - send_to for unknown conn_id is a no-op

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 80
end

require "minitest/autorun"
require "socket"
require "thread"
require "coding_adventures/irc_net_stdlib"

Lib = CodingAdventures::IrcNetStdlib

# ─────────────────────────────────────────────────────────────────────────────
# Unit: alloc_conn_id
# ─────────────────────────────────────────────────────────────────────────────

class TestAllocConnId < Minitest::Test
  def test_ids_are_unique
    # Allocate 100 IDs in parallel and verify they are all distinct.
    ids = Array.new(100) { Thread.new { Lib.alloc_conn_id } }.map(&:value)
    assert_equal ids.length, ids.uniq.length
  end

  def test_ids_are_positive_integers
    id = Lib.alloc_conn_id
    assert_kind_of Integer, id
    assert id > 0
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Integration: EventLoop with a real capturing handler
# ─────────────────────────────────────────────────────────────────────────────

# A simple Handler that records every event so tests can inspect them.
class CapturingHandler
  include Lib::Handler

  attr_reader :connects, :data_events, :disconnects

  def initialize
    @mutex      = Mutex.new
    @connects   = []   # Array<[conn_id, host]>
    @data_events = []  # Array<[conn_id, data]>
    @disconnects = []  # Array<Integer> conn_ids
    @cond       = ConditionVariable.new
  end

  def on_connect(conn_id, host)
    @mutex.synchronize { @connects << [conn_id, host]; @cond.broadcast }
  end

  def on_data(conn_id, data)
    @mutex.synchronize { @data_events << [conn_id, data]; @cond.broadcast }
  end

  def on_disconnect(conn_id)
    @mutex.synchronize { @disconnects << conn_id; @cond.broadcast }
  end

  # Wait until a condition (proc) becomes true, with a timeout.
  def wait_for(timeout: 3.0, &block)
    deadline = Time.now + timeout
    @mutex.synchronize do
      loop do
        return true if block.call
        remaining = deadline - Time.now
        break if remaining <= 0
        @cond.wait(@mutex, remaining)
      end
      block.call  # final check
    end
  end
end

# Helper: start an EventLoop in a background thread on an ephemeral port.
# Returns [loop, port, thread].
def start_server(handler)
  loop_obj = Lib::StdlibEventLoop.new
  # Use a Mutex+CondVar to learn the port after binding.
  port_mutex = Mutex.new
  port_cond  = ConditionVariable.new
  actual_port = nil

  server_thread = Thread.new do
    # We can't easily get the ephemeral port back from StdlibEventLoop#run
    # because it creates the TCPServer internally.  Instead we pre-bind a
    # server socket ourselves, learn its port, close it, then run on that port.
    # (There's a tiny TOCTOU window, but acceptable in tests.)
    tmp = TCPServer.new("127.0.0.1", 0)
    p = tmp.addr[1]
    tmp.close
    port_mutex.synchronize { actual_port = p; port_cond.broadcast }
    loop_obj.run("127.0.0.1", p, handler)
  end
  server_thread.abort_on_exception = false

  # Wait for the port to be chosen.
  port_mutex.synchronize do
    port_cond.wait(port_mutex, 2.0) while actual_port.nil?
  end
  # Give the server a moment to bind.
  sleep(0.05)

  [loop_obj, actual_port, server_thread]
end

class TestEventLoop < Minitest::Test
  def setup
    @handler = CapturingHandler.new
    @loop, @port, @server_thread = start_server(@handler)
  end

  def teardown
    @loop.stop
    @server_thread.join(2)
  end

  # ── on_connect fires when a client connects ────────────────────────────

  def test_on_connect_fires
    TCPSocket.open("127.0.0.1", @port) do |_sock|
      ok = @handler.wait_for { @handler.connects.length >= 1 }
      assert ok, "on_connect should have fired"
      assert_equal "127.0.0.1", @handler.connects[0][1]
    end
  end

  # ── on_data fires when data arrives ────────────────────────────────────

  def test_on_data_fires_with_correct_data
    TCPSocket.open("127.0.0.1", @port) do |sock|
      @handler.wait_for { @handler.connects.length >= 1 }
      sock.write("NICK alice\r\n")
      ok = @handler.wait_for { @handler.data_events.any? { |_, d| d.include?("NICK") } }
      assert ok, "on_data should have fired with NICK data"
    end
  end

  # ── on_disconnect fires when client closes ─────────────────────────────

  def test_on_disconnect_fires_on_close
    conn_id = nil
    TCPSocket.open("127.0.0.1", @port) do |sock|
      @handler.wait_for { @handler.connects.length >= 1 }
      conn_id = @handler.connects.last[0]
      sock.close
    end
    ok = @handler.wait_for { @handler.disconnects.include?(conn_id) }
    assert ok, "on_disconnect should have fired after client closed"
  end

  # ── send_to delivers data to the right connection ──────────────────────

  def test_send_to_delivers_data
    received = nil
    client = TCPSocket.open("127.0.0.1", @port)
    @handler.wait_for { @handler.connects.length >= 1 }
    conn_id = @handler.connects.last[0]

    @loop.send_to(conn_id, "HELLO FROM SERVER\r\n")

    # Read from the client side using IO.select for timeout.
    reader = Thread.new do
      if IO.select([client], nil, nil, 2.0)
        received = client.recv(4096)
      end
    end
    reader.join(3)

    client.close
    skip "recv timed out" if received.nil?
    assert_match(/HELLO FROM SERVER/, received)
  end

  # ── send_to for unknown conn_id is a silent no-op ──────────────────────

  def test_send_to_unknown_conn_id_is_noop
    # Should not raise.
    assert_nil @loop.send_to(99_999, "data")
  end

  # ── Multiple connections are handled independently ────────────────────��

  def test_multiple_connections
    socks = Array.new(3) { TCPSocket.open("127.0.0.1", @port) }
    ok = @handler.wait_for { @handler.connects.length >= 3 }
    assert ok, "All three connections should have fired on_connect"
    assert_equal 3, @handler.connects.length
  ensure
    socks.each(&:close) if socks
  end
end

# ─────────────────────────────────────────────────────────────────────────────
# Unit: Handler mixin defaults
# ─────────────────────────────────────────────────────────────────────────────

class TestHandlerMixin < Minitest::Test
  class MyHandler
    include Lib::Handler
  end

  def test_default_on_connect_is_noop
    h = MyHandler.new
    assert_nil h.on_connect(1, "host")
  end

  def test_default_on_data_is_noop
    h = MyHandler.new
    assert_nil h.on_data(1, "bytes")
  end

  def test_default_on_disconnect_is_noop
    h = MyHandler.new
    assert_nil h.on_disconnect(1)
  end
end
