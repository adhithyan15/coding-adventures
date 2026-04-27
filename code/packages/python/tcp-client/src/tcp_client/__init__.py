"""tcp-client -- TCP client with buffered I/O and configurable timeouts.

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.

Analogy: A telephone call
--------------------------

Making a TCP connection is like making a phone call:

1. DIAL (DNS + connect)
   Look up "Grandma" in contacts -> 555-0123   (DNS resolution)
   Dial and wait for ring                       (TCP three-way handshake)
   If nobody picks up -> hang up                (connect timeout)

2. TALK (read/write)
   Say "Hello, Grandma!"                        (write_all + flush)
   Listen for response                          (read_line)
   If silence for 30s -> "Still there?"         (read timeout)

3. HANG UP (shutdown/close)
   Say "Goodbye" and hang up                    (shutdown_write + close)

Where it fits
--------------

url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                          |
                     raw byte stream

Example
--------

>>> from tcp_client import connect
>>> conn = connect("info.cern.ch", 80)
>>> conn.write_all(b"GET / HTTP/1.0\\r\\nHost: info.cern.ch\\r\\n\\r\\n")
>>> conn.flush()
>>> status_line = conn.read_line()
>>> print(status_line)
'HTTP/1.0 200 OK\\r\\n'
"""

from __future__ import annotations

__version__ = "0.1.0"

import contextlib
import io
import socket

# ============================================================================
# Error hierarchy -- structured error types for TCP failures
# ============================================================================
#
# Each exception maps to a specific failure mode. Callers can catch the
# specific subclass they care about, or catch the base TcpError for all.
#
# Exception hierarchy:
#
#   TcpError (base)
#     +-- DnsResolutionFailed    hostname typo or no internet
#     +-- ConnectionRefused      server up but nothing listening on that port
#     +-- Timeout                took too long (connect, read, or write)
#     +-- ConnectionReset        remote side crashed (TCP RST)
#     +-- BrokenPipe             tried to write after remote closed
#     +-- UnexpectedEof          connection closed before expected data arrived


class TcpError(Exception):
    """Base exception for all TCP client errors."""


class DnsResolutionFailed(TcpError):
    """DNS lookup failed -- hostname could not be resolved.

    Attributes:
        host: The hostname that failed to resolve.
        message: The underlying OS error message.
    """

    def __init__(self, host: str, message: str) -> None:
        self.host = host
        self.message = message
        super().__init__(f"DNS resolution failed for '{host}': {message}")


class ConnectionRefused(TcpError):
    """Server is reachable but nothing is listening on the port (TCP RST).

    Attributes:
        addr: The address that refused the connection (e.g. "127.0.0.1:8080").
    """

    def __init__(self, addr: str) -> None:
        self.addr = addr
        super().__init__(f"connection refused by {addr}")


class Timeout(TcpError):
    """Operation timed out.

    Attributes:
        phase: Which phase timed out -- "connect", "read", or "write".
        duration: How long we waited before giving up, in seconds.
    """

    def __init__(self, phase: str, duration: float) -> None:
        self.phase = phase
        self.duration = duration
        super().__init__(f"{phase} timed out after {duration:.1f}s")


class ConnectionReset(TcpError):
    """Remote side reset the connection unexpectedly (TCP RST during transfer)."""

    def __init__(self) -> None:
        super().__init__("connection reset by peer")


class BrokenPipe(TcpError):
    """Tried to write to a connection the remote side already closed."""

    def __init__(self) -> None:
        super().__init__("broken pipe (remote closed)")


class UnexpectedEof(TcpError):
    """Connection closed before the expected number of bytes arrived.

    Attributes:
        expected: Number of bytes we asked for.
        received: Number of bytes we actually got before EOF.
    """

    def __init__(self, expected: int, received: int) -> None:
        self.expected = expected
        self.received = received
        super().__init__(
            f"unexpected EOF: expected {expected} bytes, got {received}"
        )


# ============================================================================
# ConnectOptions -- configuration for establishing a connection
# ============================================================================
#
# All timeouts default to 30 seconds. The buffer size defaults to 8192
# bytes (8 KiB), a good balance between memory usage and syscall reduction.
#
# Why separate timeouts?
#
#   connect_timeout (30s) -- how long to wait for the TCP handshake
#     If a server is down or firewalled, the OS might wait minutes.
#
#   read_timeout (30s) -- how long to wait for data after calling read
#     Without this, a stalled server hangs your program forever.
#
#   write_timeout (30s) -- how long to wait for the OS send buffer
#     Usually instant, but blocks if the remote side isn't reading.


class ConnectOptions:
    """Configuration for establishing a TCP connection.

    Attributes:
        connect_timeout: Maximum time (seconds) to wait for the TCP handshake.
        read_timeout: Maximum time (seconds) to wait for data on read.
            None means block forever.
        write_timeout: Maximum time (seconds) to wait on write.
            None means block forever.
        buffer_size: Size of internal read buffer in bytes.
    """

    def __init__(
        self,
        *,
        connect_timeout: float = 30.0,
        read_timeout: float | None = 30.0,
        write_timeout: float | None = 30.0,
        buffer_size: int = 8192,
    ) -> None:
        self.connect_timeout = connect_timeout
        self.read_timeout = read_timeout
        self.write_timeout = write_timeout
        self.buffer_size = buffer_size

    def __repr__(self) -> str:
        return (
            f"ConnectOptions(connect_timeout={self.connect_timeout}, "
            f"read_timeout={self.read_timeout}, "
            f"write_timeout={self.write_timeout}, "
            f"buffer_size={self.buffer_size})"
        )


# ============================================================================
# _map_socket_error -- translate Python socket/OS errors to our hierarchy
# ============================================================================
#
# Python's socket module raises different exception types for different
# failure modes. This function normalizes them into our TcpError hierarchy.
#
# Mapping table:
#
#   Python exception               -> TcpError subclass
#   -------------------------------------------------------
#   socket.gaierror                -> DnsResolutionFailed
#   ConnectionRefusedError         -> ConnectionRefused
#   socket.timeout / TimeoutError  -> Timeout
#   ConnectionResetError           -> ConnectionReset
#   BrokenPipeError                -> BrokenPipe
#   ConnectionAbortedError         -> ConnectionReset
#   Other OSError                  -> TcpError (generic)


def _map_socket_error(
    err: OSError,
    *,
    phase: str = "io",
    duration: float = 0.0,
    host: str = "",
    addr: str = "",
) -> TcpError:
    """Convert a Python socket/OS error into the appropriate TcpError subclass.

    Args:
        err: The original OS-level error.
        phase: Which phase we were in ("connect", "read", "write", "io").
        duration: How long the timeout was set to, if applicable.
        host: The hostname, used for DNS errors.
        addr: The address string, used for connection refused.

    Returns:
        A TcpError subclass instance.
    """
    # DNS resolution failure: socket.gaierror is a subclass of OSError.
    # We check this first because gaierror is also an OSError.
    if isinstance(err, socket.gaierror):
        return DnsResolutionFailed(host=host, message=str(err))

    # Connection refused: the server sent TCP RST during handshake
    if isinstance(err, ConnectionRefusedError):
        return ConnectionRefused(addr=addr)

    # Timeout: the OS-level timer expired
    if isinstance(err, (socket.timeout, TimeoutError)):
        return Timeout(phase=phase, duration=duration)

    # Connection reset: remote side crashed or sent RST during transfer
    if isinstance(err, (ConnectionResetError, ConnectionAbortedError)):
        return ConnectionReset()

    # Broken pipe: tried to write after the remote side closed
    if isinstance(err, BrokenPipeError):
        return BrokenPipe()

    # Fallback: wrap in a generic TcpError
    return TcpError(f"I/O error: {err}")


# ============================================================================
# connect() -- establish a TCP connection
# ============================================================================
#
# Algorithm:
#
# 1. DNS resolution: (host, port) -> [addr1, addr2, ...]
#    Uses the OS resolver (respects /etc/hosts, system DNS).
#    socket.create_connection handles this internally.
#
# 2. Connect with timeout:
#    socket.create_connection tries each resolved address in order,
#    similar to the "Happy Eyeballs" algorithm.
#
# 3. Configure the connected socket:
#    set socket.settimeout() for read/write operations.
#
# 4. Wrap in socket.makefile('rb') for buffered reading.


def connect(
    host: str,
    port: int,
    options: ConnectOptions | None = None,
) -> TcpConnection:
    """Establish a TCP connection to the given host and port.

    Args:
        host: Hostname or IP address to connect to.
        port: Port number (1-65535).
        options: Connection configuration. Uses defaults if None.

    Returns:
        A TcpConnection ready for reading and writing.

    Raises:
        DnsResolutionFailed: If the hostname cannot be resolved.
        ConnectionRefused: If nothing is listening on the target port.
        Timeout: If the connection attempt takes too long.
        TcpError: For other connection failures.

    Example:
        >>> conn = connect("example.com", 80)
        >>> conn.write_all(b"GET / HTTP/1.0\\r\\n\\r\\n")
        >>> conn.flush()
        >>> print(conn.read_line())
    """
    if options is None:
        options = ConnectOptions()

    try:
        # socket.create_connection handles DNS resolution and tries each
        # resolved address in sequence. The timeout applies to the entire
        # connect attempt (DNS + TCP handshake).
        sock = socket.create_connection(
            (host, port),
            timeout=options.connect_timeout,
        )
    except socket.gaierror as e:
        raise DnsResolutionFailed(host=host, message=str(e)) from e
    except ConnectionRefusedError as e:
        raise ConnectionRefused(addr=f"{host}:{port}") from e
    except TimeoutError as e:
        raise Timeout(phase="connect", duration=options.connect_timeout) from e
    except OSError as e:
        raise _map_socket_error(
            e, phase="connect", duration=options.connect_timeout,
            host=host, addr=f"{host}:{port}",
        ) from e

    # Configure read/write timeout on the connected socket.
    #
    # Python sockets use a single timeout for both read and write. We use
    # the read_timeout here since reads are more commonly the blocking
    # operation. For truly separate timeouts, you would need select() or
    # asyncio, which is beyond this package's scope.
    #
    # If read_timeout is None, the socket blocks forever on reads.
    sock.settimeout(options.read_timeout)

    # Wrap the socket in a buffered file object for efficient reading.
    #
    # socket.makefile('rb') creates a BufferedReader backed by the socket.
    # This gives us readline() and read(n) for free -- no manual buffering
    # needed. The buffering parameter sets the internal buffer size.
    #
    # Why buffered I/O?
    #
    #   Without buffering:
    #     recv() returns arbitrary chunks: b"HT", b"TP/", b"1.0 2", b"00 OK\r\n"
    #     100 recv() calls = 100 syscalls (expensive!)
    #
    #   With makefile (8 KiB internal buffer):
    #     First read pulls 8 KiB from the OS into memory
    #     Subsequent readline() calls serve from the buffer
    #     100 lines might need only 1-2 syscalls
    reader = sock.makefile("rb", buffering=options.buffer_size)

    return TcpConnection(sock=sock, reader=reader, options=options)


# ============================================================================
# TcpConnection -- buffered I/O over a TCP stream
# ============================================================================
#
# Wraps a connected socket with a buffered reader for efficient,
# line-oriented or chunk-oriented communication. Writing goes directly
# through socket.sendall() which is already atomic and complete.
#
# The connection is automatically closeable via close() or a context manager.


class TcpConnection:
    """A TCP connection with buffered reading and configured timeouts.

    This class wraps a raw socket with buffered read operations for
    efficient line-oriented and binary protocols. Writing uses sendall()
    which guarantees all bytes are sent or an error is raised.

    Usage:
        conn = connect("example.com", 80)
        conn.write_all(b"GET / HTTP/1.0\\r\\n\\r\\n")
        conn.flush()
        line = conn.read_line()
        conn.close()
    """

    def __init__(
        self,
        *,
        sock: socket.socket,
        reader: io.IOBase,
        options: ConnectOptions,
    ) -> None:
        self._sock = sock
        self._reader = reader
        self._options = options

    def __repr__(self) -> str:
        try:
            peer = self.peer_addr()
            local = self.local_addr()
            return f"TcpConnection(peer={peer}, local={local})"
        except TcpError:
            return "TcpConnection(closed)"

    def __enter__(self) -> TcpConnection:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        self.close()

    # ── Reading methods ─────────────────────────────────────────────────

    def read_line(self) -> str:
        """Read bytes until a newline (\\n) is found.

        Returns the line *including* the trailing \\n (and \\r\\n if present).
        Returns an empty string at EOF (remote closed cleanly).

        This is the workhorse for line-oriented protocols like HTTP/1.0,
        SMTP, and RESP (Redis protocol).

        Returns:
            The line as a string, or "" at EOF.

        Raises:
            Timeout: If no data arrives within the read timeout.
            ConnectionReset: If the remote side closed unexpectedly.
        """
        try:
            line: bytes = self._reader.readline()
        except OSError as e:
            raise _map_socket_error(
                e, phase="read", duration=self._options.read_timeout or 0.0,
            ) from e

        # EOF: readline() returns b"" when the connection is closed cleanly.
        # We return "" (empty string) to signal this to the caller.
        if not line:
            return ""

        # Decode the bytes to a string. We use latin-1 because it maps
        # byte values 0-255 directly to Unicode code points 0-255, so
        # no byte is ever invalid. For actual text protocols, callers
        # can re-encode as needed.
        return line.decode("latin-1")

    def read_exact(self, n: int) -> bytes:
        """Read exactly n bytes from the connection.

        Blocks until all n bytes have been received. Useful for protocols
        that specify an exact content length (e.g., HTTP Content-Length).

        Args:
            n: The exact number of bytes to read.

        Returns:
            A bytes object of exactly n bytes.

        Raises:
            UnexpectedEof: If the connection closes before n bytes arrive.
            Timeout: If the read times out.
        """
        try:
            data: bytes = self._reader.read(n)
        except OSError as e:
            raise _map_socket_error(
                e, phase="read", duration=self._options.read_timeout or 0.0,
            ) from e

        if data is None:
            data = b""

        # If we got fewer bytes than requested, the connection closed early.
        # This is an "unexpected EOF" -- the sender promised more data than
        # it delivered.
        if len(data) < n:
            raise UnexpectedEof(expected=n, received=len(data))

        return data

    def read_until(self, delimiter: int) -> bytes:
        """Read bytes until the given delimiter byte is found.

        Returns all bytes up to *and including* the delimiter. Useful for
        protocols with custom delimiters (RESP uses \\r\\n, null-terminated
        strings use \\0).

        Args:
            delimiter: The byte value to stop at (0-255).

        Returns:
            Bytes up to and including the delimiter.

        Raises:
            Timeout: If the read times out.
            ConnectionReset: If the connection is reset.
        """
        # We read one byte at a time from the buffered reader. This sounds
        # slow, but the BufferedReader has an internal 8 KiB buffer, so most
        # of these reads are just memcpy from the buffer -- not syscalls.
        result = bytearray()
        while True:
            try:
                byte: bytes = self._reader.read(1)
            except OSError as e:
                raise _map_socket_error(
                    e, phase="read",
                    duration=self._options.read_timeout or 0.0,
                ) from e

            if not byte:
                # EOF before finding delimiter -- return what we have
                break

            result.extend(byte)

            if byte[0] == delimiter:
                break

        return bytes(result)

    # ── Writing methods ─────────────────────────────────────────────────

    def write_all(self, data: bytes) -> None:
        """Write all bytes to the connection.

        Uses socket.sendall() which guarantees that either all bytes are
        sent or an error is raised. There is no partial write.

        Args:
            data: The bytes to send.

        Raises:
            BrokenPipe: If the remote side has closed the connection.
            Timeout: If the write times out.
        """
        try:
            self._sock.sendall(data)
        except OSError as e:
            raise _map_socket_error(
                e, phase="write",
                duration=self._options.write_timeout or 0.0,
            ) from e

    def flush(self) -> None:
        """Flush the write buffer.

        Since we use socket.sendall() directly (which is already immediate
        and complete), this is a no-op. It exists for API compatibility
        with the Rust implementation which uses BufWriter.
        """
        # No-op: sendall() already sends everything immediately.
        # If we ever switch to buffered writing, this would flush the buffer.

    # ── Connection management ───────────────────────────────────────────

    def shutdown_write(self) -> None:
        """Shut down the write half of the connection (half-close).

        Signals to the remote side that no more data will be sent. The
        read half remains open -- you can still receive data.

        Before shutdown_write():
          Client <-> Server  (full-duplex, both directions open)

        After shutdown_write():
          Client <- Server   (client can still READ)
          Client x Server    (client can no longer WRITE)

        Raises:
            TcpError: If the shutdown fails.
        """
        try:
            self._sock.shutdown(socket.SHUT_WR)
        except OSError as e:
            raise _map_socket_error(e) from e

    def peer_addr(self) -> tuple[str, int]:
        """Return the remote address (IP and port) of this connection.

        Returns:
            A (host, port) tuple.

        Raises:
            TcpError: If the socket is closed or invalid.
        """
        try:
            addr: tuple[str, int] = self._sock.getpeername()[:2]
            return addr
        except OSError as e:
            raise _map_socket_error(e) from e

    def local_addr(self) -> tuple[str, int]:
        """Return the local address (IP and port) of this connection.

        Returns:
            A (host, port) tuple.

        Raises:
            TcpError: If the socket is closed or invalid.
        """
        try:
            addr: tuple[str, int] = self._sock.getsockname()[:2]
            return addr
        except OSError as e:
            raise _map_socket_error(e) from e

    def close(self) -> None:
        """Close the connection, releasing all resources.

        Safe to call multiple times. After closing, all read/write
        operations will fail.
        """
        with contextlib.suppress(OSError):
            self._reader.close()
        with contextlib.suppress(OSError):
            self._sock.close()
