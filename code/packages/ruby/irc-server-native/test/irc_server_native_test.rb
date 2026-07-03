# frozen_string_literal: true

require "minitest/autorun"
require "socket"
require "coding_adventures_irc_server_native"

# End-to-end tests: start the real Rust IRC engine on an ephemeral port and
# drive live IRC clients over real TCP sockets.
class IrcServerNativeTest < Minitest::Test
  Error = CodingAdventures::IrcServerNative::Error
  IrcServer = CodingAdventures::IrcServerNative::IrcServer
  NativeServer = CodingAdventures::IrcServerNative::NativeServer

  def setup
    @server = IrcServer.new(host: "127.0.0.1", port: 0, server_name: "irc.test")
    @server.start
    @port = @server.local_port
  end

  def teardown
    @server&.close
  end

  # ── helpers ────────────────────────────────────────────────────────────────

  def recv_until(sock, needle, timeout: 5.0)
    deadline = Time.now + timeout
    buf = +""
    while Time.now < deadline
      next unless IO.select([sock], nil, nil, 0.3)

      begin
        buf << sock.read_nonblock(4096)
      rescue IO::WaitReadable
        next
      rescue EOFError
        break
      end
      break if buf.include?(needle)
    end
    buf
  end

  def connect
    TCPSocket.new("127.0.0.1", @port)
  end

  def register(sock, nick)
    sock.write("NICK #{nick}\r\nUSER #{nick} 0 * :#{nick}\r\n")
    welcome = recv_until(sock, "001")
    assert_includes welcome, "001", "expected 001 welcome for #{nick}"
  end

  # ── tests ──────────────────────────────────────────────────────────────────

  def test_local_addr_and_running
    assert_equal "127.0.0.1", @server.local_host
    assert_operator @server.local_port, :>, 0
    assert @server.running?
    assert_equal "127.0.0.1:#{@port}", @server.local_addr
  end

  def test_registration_and_ping
    alice = connect
    register(alice, "alice")
    alice.write("PING :liveness\r\n")
    pong = recv_until(alice, "PONG")
    assert_includes pong, "PONG"
  ensure
    alice&.close
  end

  def test_privmsg_broadcasts_between_clients
    alice = connect
    bob = connect
    register(alice, "alice")
    register(bob, "bob")
    alice.write("JOIN #test\r\n")
    bob.write("JOIN #test\r\n")
    recv_until(alice, "JOIN")
    recv_until(bob, "JOIN")

    # Alice speaks; Bob (a different connection) must receive it — exercises the
    # Rust engine's in-process mailbox fan-out.
    alice.write("PRIVMSG #test :hello bob\r\n")
    received = recv_until(bob, "hello bob")
    assert_includes received, "PRIVMSG"
    assert_includes received, "hello bob"
  ensure
    alice&.close
    bob&.close
  end

  def test_quit_broadcasts_to_channel
    alice = connect
    bob = connect
    register(alice, "alice")
    register(bob, "bob")
    alice.write("JOIN #test\r\n")
    bob.write("JOIN #test\r\n")
    recv_until(bob, "JOIN")

    alice.write("QUIT :leaving now\r\n")
    quit = recv_until(bob, "QUIT")
    assert_includes quit, "QUIT"
  ensure
    alice&.close
    bob&.close
  end

  # The native dispose frees the engine; doing so mid-serve would be a
  # use-after-free, so it must be refused while the loop is running.
  def test_native_dispose_refused_while_running
    native = NativeServer.new("127.0.0.1", 0, "irc.test", ["hi"], "", 1024)
    thread = Thread.new { native.serve }
    100.times { break if native.running?; sleep 0.01 }
    assert native.running?, "native server should be running"

    err = assert_raises(Error) { native.dispose }
    assert_match(/running/, err.message)
  ensure
    native&.stop
    thread&.join(5)
    native&.dispose
  end
end
