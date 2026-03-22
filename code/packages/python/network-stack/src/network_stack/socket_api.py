"""
Socket API — The BSD Socket Interface
======================================

The socket API is the interface between applications and the networking stack.
When a program wants to communicate over the network, it doesn't talk to TCP
or UDP directly — it uses sockets.

What is a Socket?
-----------------

A socket is an **endpoint** for network communication, identified by a file
descriptor (an integer). Just like a file descriptor lets you read/write files,
a socket descriptor lets you send/receive network data.

The analogy: a socket is like a phone. You can:

1. **Create** a phone (``socket()``) — pick up a new phone
2. **Bind** it to a number (``bind()``) — get a phone number
3. **Listen** for calls (``listen()``) — turn on the ringer (TCP server)
4. **Accept** a call (``accept()``) — pick up when someone calls (TCP server)
5. **Connect** to someone (``connect()``) — dial a number (TCP client)
6. **Send** data (``send()``) — talk into the phone
7. **Receive** data (``recv()``) — listen to the phone
8. **Close** the phone (``close()``) — hang up

Two Types of Sockets
--------------------

- **STREAM** (TCP): A reliable, ordered, bidirectional byte stream.
  Like a phone call — you establish a connection, talk back and forth,
  and hang up when done.

- **DGRAM** (UDP): Unreliable, unordered datagrams.
  Like sending a text message — each message is independent, no
  connection needed, and messages might get lost.

The SocketManager
-----------------

In a real OS, the kernel manages all sockets. Our ``SocketManager`` plays
that role. It assigns file descriptors, creates the underlying TCP connections
or UDP sockets, and routes calls to the right protocol handler.
"""

from __future__ import annotations

from enum import IntEnum

from network_stack.tcp import TCPConnection
from network_stack.udp import UDPSocket


class SocketType(IntEnum):
    """
    The type of socket, which determines the transport protocol.

    STREAM sockets use TCP (reliable, ordered connections).
    DGRAM sockets use UDP (unreliable, connectionless datagrams).
    """

    STREAM = 1  # TCP — like a phone call
    DGRAM = 2   # UDP — like a text message


class Socket:
    """
    A single network socket — one endpoint of communication.

    This is the kernel's internal representation of a socket. Applications
    interact with sockets through the SocketManager using file descriptors;
    they never touch Socket objects directly.

    Attributes
    ----------
    socket_type : SocketType
        Whether this is a TCP (STREAM) or UDP (DGRAM) socket.
    fd : int
        File descriptor assigned by the SocketManager. -1 until assigned.
    bound_ip : int
        The IP address this socket is bound to. 0 = any address.
    bound_port : int
        The port number this socket is bound to. 0 = not yet bound.
    tcp_connection : TCPConnection | None
        The underlying TCP connection (only for STREAM sockets).
    udp_socket : UDPSocket | None
        The underlying UDP socket (only for DGRAM sockets).
    listening : bool
        Whether this socket is listening for incoming connections (TCP only).
    accept_queue : list[TCPConnection]
        Queue of incoming connections waiting to be accepted (TCP server).
    """

    def __init__(self, socket_type: SocketType) -> None:
        self.socket_type = socket_type
        self.fd = -1
        self.bound_ip = 0
        self.bound_port = 0
        self.tcp_connection: TCPConnection | None = None
        self.udp_socket: UDPSocket | None = None
        self.listening = False
        self.accept_queue: list[TCPConnection] = []


class SocketManager:
    """
    Manages all sockets in the system — the kernel's socket table.

    This is the single point of entry for all socket operations. It maintains
    a table mapping file descriptors to Socket objects and routes each
    operation to the appropriate protocol handler.

    File Descriptors
    ----------------

    File descriptors are small integers that identify open files, sockets,
    pipes, etc. In Unix, everything is a file — including network connections.
    We start at fd=10 to avoid colliding with stdin (0), stdout (1), and
    stderr (2).

    Return Values
    -------------

    Most methods return 0 on success and -1 on error, following the Unix
    convention. This is how the kernel communicates errors to user programs
    through syscalls.
    """

    def __init__(self) -> None:
        self._sockets: dict[int, Socket] = {}
        self._next_fd = 10  # Start above stdin/stdout/stderr

    def socket(self, socket_type: SocketType) -> int:
        """
        Create a new socket and return its file descriptor.

        This is equivalent to the Unix ``socket()`` syscall::

            int fd = socket(AF_INET, SOCK_STREAM, 0);

        Parameters
        ----------
        socket_type : SocketType
            STREAM (TCP) or DGRAM (UDP).

        Returns
        -------
        int
            The file descriptor for the new socket.
        """
        sock = Socket(socket_type)
        fd = self._next_fd
        self._next_fd += 1
        sock.fd = fd
        self._sockets[fd] = sock
        return fd

    def bind(self, fd: int, ip: int, port: int) -> int:
        """
        Bind a socket to a specific IP address and port.

        This assigns a "name" (address + port) to the socket. Servers
        must bind before they can listen for connections.

        Returns 0 on success, -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None:
            return -1

        sock.bound_ip = ip
        sock.bound_port = port

        # Set up the underlying protocol handler
        if sock.socket_type == SocketType.DGRAM:
            sock.udp_socket = UDPSocket(local_port=port)
        elif sock.socket_type == SocketType.STREAM:
            sock.tcp_connection = TCPConnection(local_port=port)

        return 0

    def listen(self, fd: int, backlog: int = 5) -> int:
        """
        Mark a TCP socket as a passive listener, ready to accept connections.

        The ``backlog`` parameter sets the maximum number of pending
        connections in the accept queue. We don't enforce this limit in
        our simulation.

        Returns 0 on success, -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.socket_type != SocketType.STREAM:
            return -1

        sock.listening = True
        if sock.tcp_connection is not None:
            sock.tcp_connection.initiate_listen()

        return 0

    def accept(self, fd: int) -> int | None:
        """
        Accept a pending connection from the accept queue.

        Creates a new socket for the accepted connection and returns its
        file descriptor. Returns None if no connections are pending.

        In a real OS, ``accept()`` would block until a connection arrives.
        In our simulation, it returns None immediately if the queue is empty.
        """
        sock = self.get_socket(fd)
        if sock is None or not sock.listening:
            return None

        if not sock.accept_queue:
            return None

        conn = sock.accept_queue.pop(0)
        new_fd = self.socket(SocketType.STREAM)
        new_sock = self._sockets[new_fd]
        new_sock.tcp_connection = conn
        new_sock.bound_ip = sock.bound_ip
        new_sock.bound_port = sock.bound_port
        return new_fd

    def connect(self, fd: int, ip: int, port: int) -> int:
        """
        Initiate a TCP connection to a remote host.

        This begins the three-way handshake. In our simulation, we just
        set up the TCPConnection and send the SYN. The caller must
        complete the handshake by processing the SYN+ACK.

        Returns 0 on success, -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.socket_type != SocketType.STREAM:
            return -1

        if sock.tcp_connection is None:
            sock.tcp_connection = TCPConnection(
                local_port=sock.bound_port,
                remote_ip=ip,
                remote_port=port,
            )
        else:
            sock.tcp_connection.remote_ip = ip
            sock.tcp_connection.remote_port = port

        sock.tcp_connection.initiate_connect()
        return 0

    def send(self, fd: int, data: bytes) -> int:
        """
        Send data over a connected TCP socket.

        Returns the number of bytes queued for sending, or -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.tcp_connection is None:
            return -1

        sock.tcp_connection.send(data)
        return len(data)

    def recv(self, fd: int, count: int) -> bytes:
        """
        Receive data from a connected TCP socket.

        Returns up to ``count`` bytes from the receive buffer.
        Returns empty bytes if no data is available.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.tcp_connection is None:
            return b""

        return sock.tcp_connection.receive(count)

    def sendto(self, fd: int, data: bytes, ip: int, port: int) -> int:
        """
        Send a UDP datagram to a specific destination.

        Unlike TCP's ``send()``, you must specify the destination with
        every call because UDP is connectionless.

        Returns the number of bytes sent, or -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.udp_socket is None:
            return -1

        sock.udp_socket.send_to(data, ip, port)
        return len(data)

    def recvfrom(self, fd: int) -> tuple[bytes, int, int] | None:
        """
        Receive a UDP datagram, returning (data, src_ip, src_port).

        Returns None if no datagrams are available.
        """
        sock = self.get_socket(fd)
        if sock is None or sock.udp_socket is None:
            return None

        return sock.udp_socket.receive_from()

    def close(self, fd: int) -> int:
        """
        Close a socket and release its file descriptor.

        For TCP sockets, this initiates the connection teardown (FIN).
        For UDP sockets, it simply removes the socket from the table.

        Returns 0 on success, -1 on error.
        """
        sock = self.get_socket(fd)
        if sock is None:
            return -1

        if sock.tcp_connection is not None:
            sock.tcp_connection.initiate_close()

        del self._sockets[fd]
        return 0

    def get_socket(self, fd: int) -> Socket | None:
        """
        Look up a socket by file descriptor.

        Returns None if the fd doesn't refer to an open socket.
        """
        return self._sockets.get(fd)
