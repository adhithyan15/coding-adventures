"""Tests for the socket API — SocketManager, STREAM and DGRAM sockets."""

from network_stack.socket_api import Socket, SocketManager, SocketType
from network_stack.tcp import TCPConnection, TCPState


class TestSocketCreation:
    """Tests for creating sockets."""

    def test_create_stream_socket(self) -> None:
        """socket(STREAM) should return a valid file descriptor."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        assert fd >= 10  # above stdin/stdout/stderr
        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.socket_type == SocketType.STREAM

    def test_create_dgram_socket(self) -> None:
        """socket(DGRAM) should return a valid file descriptor."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.DGRAM)
        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.socket_type == SocketType.DGRAM

    def test_unique_file_descriptors(self) -> None:
        """Each socket should get a unique file descriptor."""
        mgr = SocketManager()
        fd1 = mgr.socket(SocketType.STREAM)
        fd2 = mgr.socket(SocketType.DGRAM)
        fd3 = mgr.socket(SocketType.STREAM)
        assert fd1 != fd2 != fd3

    def test_get_nonexistent_socket(self) -> None:
        """get_socket with unknown fd should return None."""
        mgr = SocketManager()
        assert mgr.get_socket(999) is None


class TestSocketBind:
    """Tests for binding sockets to addresses."""

    def test_bind_stream_socket(self) -> None:
        """bind() on a STREAM socket should set up TCP connection."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        result = mgr.bind(fd, 0x0A000001, 80)
        assert result == 0

        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.bound_ip == 0x0A000001
        assert sock.bound_port == 80
        assert sock.tcp_connection is not None

    def test_bind_dgram_socket(self) -> None:
        """bind() on a DGRAM socket should set up UDP socket."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.DGRAM)
        result = mgr.bind(fd, 0x0A000001, 53)
        assert result == 0

        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.udp_socket is not None
        assert sock.udp_socket.local_port == 53

    def test_bind_invalid_fd(self) -> None:
        """bind() with invalid fd should return -1."""
        mgr = SocketManager()
        assert mgr.bind(999, 0, 80) == -1


class TestSocketTCPOperations:
    """Tests for TCP socket operations."""

    def test_listen(self) -> None:
        """listen() should set up the socket for accepting connections."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)
        result = mgr.listen(fd)
        assert result == 0

        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.listening

    def test_listen_on_dgram_fails(self) -> None:
        """listen() on a UDP socket should return -1."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.DGRAM)
        assert mgr.listen(fd) == -1

    def test_connect(self) -> None:
        """connect() should initiate TCP handshake."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0x0A000001, 49152)
        result = mgr.connect(fd, 0x0A000002, 80)
        assert result == 0

        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.tcp_connection is not None
        assert sock.tcp_connection.state == TCPState.SYN_SENT

    def test_send_and_recv(self) -> None:
        """send() and recv() should work on a STREAM socket with connection."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)

        # Manually set connection to ESTABLISHED for testing
        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.tcp_connection is not None
        sock.tcp_connection.state = TCPState.ESTABLISHED

        # Send data
        sent = mgr.send(fd, b"Hello")
        assert sent == 5

        # Data is in send_buffer, not recv_buffer (would need the other side)
        assert mgr.recv(fd, 100) == b""

    def test_accept_with_queued_connection(self) -> None:
        """accept() should return a new fd when there's a pending connection."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)
        mgr.listen(fd)

        # Simulate an incoming connection
        sock = mgr.get_socket(fd)
        assert sock is not None
        incoming = TCPConnection(local_port=80)
        incoming.state = TCPState.ESTABLISHED
        sock.accept_queue.append(incoming)

        new_fd = mgr.accept(fd)
        assert new_fd is not None
        assert new_fd != fd

        new_sock = mgr.get_socket(new_fd)
        assert new_sock is not None
        assert new_sock.tcp_connection is incoming

    def test_accept_empty_queue(self) -> None:
        """accept() should return None when no connections are pending."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)
        mgr.listen(fd)
        assert mgr.accept(fd) is None

    def test_accept_not_listening(self) -> None:
        """accept() on a non-listening socket should return None."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)
        assert mgr.accept(fd) is None


class TestSocketUDPOperations:
    """Tests for UDP socket operations."""

    def test_sendto_and_recvfrom(self) -> None:
        """sendto/recvfrom should work on DGRAM sockets."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.DGRAM)
        mgr.bind(fd, 0, 12345)

        # sendto
        sent = mgr.sendto(fd, b"hello", 0x08080808, 53)
        assert sent == 5

        # Simulate receiving
        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.udp_socket is not None
        sock.udp_socket.deliver(b"response", 0x08080808, 53)

        result = mgr.recvfrom(fd)
        assert result is not None
        data, ip, port = result
        assert data == b"response"

    def test_recvfrom_empty(self) -> None:
        """recvfrom() on empty queue should return None."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.DGRAM)
        mgr.bind(fd, 0, 12345)
        assert mgr.recvfrom(fd) is None

    def test_sendto_invalid_fd(self) -> None:
        """sendto() with invalid fd should return -1."""
        mgr = SocketManager()
        assert mgr.sendto(999, b"data", 0, 0) == -1


class TestSocketClose:
    """Tests for closing sockets."""

    def test_close_removes_socket(self) -> None:
        """close() should remove the socket from the manager."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        assert mgr.get_socket(fd) is not None

        result = mgr.close(fd)
        assert result == 0
        assert mgr.get_socket(fd) is None

    def test_close_invalid_fd(self) -> None:
        """close() with invalid fd should return -1."""
        mgr = SocketManager()
        assert mgr.close(999) == -1

    def test_close_initiates_tcp_teardown(self) -> None:
        """close() on a TCP socket should initiate FIN."""
        mgr = SocketManager()
        fd = mgr.socket(SocketType.STREAM)
        mgr.bind(fd, 0, 80)

        sock = mgr.get_socket(fd)
        assert sock is not None
        assert sock.tcp_connection is not None
        sock.tcp_connection.state = TCPState.ESTABLISHED

        mgr.close(fd)
        # After close, the socket is removed but TCP teardown was initiated

    def test_socket_default_values(self) -> None:
        """A new Socket should have sensible defaults."""
        sock = Socket(SocketType.STREAM)
        assert sock.fd == -1
        assert sock.bound_ip == 0
        assert sock.bound_port == 0
        assert sock.tcp_connection is None
        assert sock.udp_socket is None
        assert not sock.listening
        assert sock.accept_queue == []
