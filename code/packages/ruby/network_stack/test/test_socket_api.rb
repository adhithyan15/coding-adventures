# frozen_string_literal: true

require_relative "test_helper"

class TestSocketAPI < Minitest::Test
  include CodingAdventures::NetworkStack

  def test_create_stream_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)

    assert fd >= 3  # 0,1,2 are stdin/stdout/stderr
    assert_equal SocketType::STREAM, mgr.sockets[fd].socket_type
  end

  def test_create_dgram_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)

    assert_equal SocketType::DGRAM, mgr.sockets[fd].socket_type
  end

  def test_bind_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)

    result = mgr.bind(fd, [10, 0, 0, 1], 8080)
    assert result

    sock = mgr.sockets[fd]
    assert_equal [10, 0, 0, 1], sock.local_ip
    assert_equal 8080, sock.local_port
  end

  def test_bind_duplicate_port_fails
    mgr = SocketManager.new
    fd1 = mgr.create_socket(SocketType::STREAM)
    fd2 = mgr.create_socket(SocketType::STREAM)

    assert mgr.bind(fd1, [10, 0, 0, 1], 80)
    refute mgr.bind(fd2, [10, 0, 0, 1], 80)
  end

  def test_bind_invalid_fd_fails
    mgr = SocketManager.new
    refute mgr.bind(999, [10, 0, 0, 1], 80)
  end

  def test_listen
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    mgr.bind(fd, [10, 0, 0, 1], 80)

    result = mgr.listen(fd)
    assert result
    assert mgr.sockets[fd].listening?
  end

  def test_listen_on_dgram_fails
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    mgr.bind(fd, [10, 0, 0, 1], 80)

    refute mgr.listen(fd)
  end

  def test_connect_stream_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)

    syn = mgr.connect(fd, [10, 0, 0, 2], 80)
    refute_nil syn

    sock = mgr.sockets[fd]
    assert_equal [10, 0, 0, 2], sock.remote_ip
    assert_equal 80, sock.remote_port
    assert sock.local_port >= 49152  # ephemeral port
  end

  def test_connect_auto_assigns_ephemeral_port
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)

    mgr.connect(fd, [10, 0, 0, 2], 80)
    assert mgr.sockets[fd].local_port >= 49152
  end

  def test_close_stream_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    mgr.bind(fd, [10, 0, 0, 1], 8080)

    result = mgr.close(fd)
    # Returns true because TCP connection is nil (not established)
    assert result
    assert_nil mgr.sockets[fd]
  end

  def test_close_invalid_fd
    mgr = SocketManager.new
    refute mgr.close(999)
  end

  def test_dgram_bind_creates_udp_socket
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    mgr.bind(fd, [10, 0, 0, 1], 5000)

    sock = mgr.sockets[fd]
    refute_nil sock.udp_socket
    assert_equal 5000, sock.udp_socket.local_port
  end

  def test_sendto_and_recvfrom_dgram
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    mgr.bind(fd, [10, 0, 0, 1], 5000)

    # Send a datagram
    result = mgr.sendto(fd, [1, 2, 3], [10, 0, 0, 2], 6000)
    refute_nil result

    # Deliver a datagram to the socket
    mgr.sockets[fd].udp_socket.deliver([4, 5, 6], [10, 0, 0, 2], 6000)

    # Receive the datagram
    received = mgr.recvfrom(fd)
    refute_nil received
    assert_equal [4, 5, 6], received[:data]
  end

  def test_sendto_on_stream_returns_nil
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    assert_nil mgr.sendto(fd, [1], [10, 0, 0, 1], 80)
  end

  def test_recvfrom_on_stream_returns_nil
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    assert_nil mgr.recvfrom(fd)
  end

  def test_accept_on_non_listening_returns_nil
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    assert_nil mgr.accept(fd)
  end

  def test_accept_with_queued_connection
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    mgr.bind(fd, [10, 0, 0, 1], 80)
    mgr.listen(fd)

    # Manually enqueue a connection
    conn = TCPConnection.new(local_port: 80, remote_ip: [10, 0, 0, 2], remote_port: 49152)
    conn.state = TCPState::ESTABLISHED
    mgr.sockets[fd].accept_queue.push({
      remote_ip: [10, 0, 0, 2],
      remote_port: 49152,
      connection: conn
    })

    new_fd, remote_ip, remote_port = mgr.accept(fd)
    refute_nil new_fd
    assert_equal [10, 0, 0, 2], remote_ip
    assert_equal 49152, remote_port

    new_sock = mgr.sockets[new_fd]
    assert_equal TCPState::ESTABLISHED, new_sock.tcp_connection.state
  end

  def test_socket_listening_predicate
    sock = Socket.new(fd: 3, socket_type: SocketType::STREAM)
    refute sock.listening?
    sock.listening = true
    assert sock.listening?
  end

  def test_listen_invalid_fd
    mgr = SocketManager.new
    refute mgr.listen(999)
  end

  def test_send_data_stream
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    mgr.connect(fd, [10, 0, 0, 2], 80)

    # Complete the handshake manually
    sock = mgr.sockets[fd]
    syn_ack = TCPHeader.new(
      src_port: 80, dst_port: sock.local_port,
      seq_num: 5000, ack_num: sock.tcp_connection.seq_num + 1,
      flags: TCP_SYN | TCP_ACK
    )
    sock.tcp_connection.handle_segment(syn_ack)

    # Now send data
    header = mgr.send_data(fd, [72, 101, 108, 108, 111])
    refute_nil header
  end

  def test_recv_stream
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::STREAM)
    mgr.connect(fd, [10, 0, 0, 2], 80)

    sock = mgr.sockets[fd]
    syn_ack = TCPHeader.new(
      src_port: 80, dst_port: sock.local_port,
      seq_num: 5000, ack_num: sock.tcp_connection.seq_num + 1,
      flags: TCP_SYN | TCP_ACK
    )
    sock.tcp_connection.handle_segment(syn_ack)

    # Deliver some data
    data_header = TCPHeader.new(
      src_port: 80, dst_port: sock.local_port,
      seq_num: 5001, ack_num: sock.tcp_connection.seq_num,
      flags: TCP_ACK | TCP_PSH
    )
    sock.tcp_connection.handle_segment(data_header, [65, 66, 67])

    received = mgr.recv(fd)
    assert_equal [65, 66, 67], received
  end

  def test_send_data_dgram
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    mgr.bind(fd, [10, 0, 0, 1], 5000)

    result = mgr.send_data(fd, [1, 2, 3])
    refute_nil result
  end

  def test_recv_dgram
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    mgr.bind(fd, [10, 0, 0, 1], 5000)

    mgr.sockets[fd].udp_socket.deliver([10, 20], [10, 0, 0, 2], 6000)
    result = mgr.recv(fd)
    refute_nil result
    assert_equal [10, 20], result[:data]
  end

  def test_send_data_nil_fd
    mgr = SocketManager.new
    assert_nil mgr.send_data(999, [1])
  end

  def test_recv_nil_fd
    mgr = SocketManager.new
    assert_nil mgr.recv(999)
  end

  def test_connect_nil_fd
    mgr = SocketManager.new
    assert_nil mgr.connect(999, [10, 0, 0, 1], 80)
  end

  def test_connect_dgram_returns_nil
    mgr = SocketManager.new
    fd = mgr.create_socket(SocketType::DGRAM)
    assert_nil mgr.connect(fd, [10, 0, 0, 1], 80)
  end
end
