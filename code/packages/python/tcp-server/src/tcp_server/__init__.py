"""
tcp_server — Single-threaded TCP server with pluggable handler.

DT24 in the coding-adventures series.

This package implements a TCP server from first principles using Python's
``socket`` and ``selectors`` modules. The server is entirely agnostic about
the protocol spoken by its clients — it reads raw bytes, hands them to a
user-supplied handler function, and writes back whatever the handler returns.

The event loop uses OS-level I/O multiplexing (epoll on Linux, kqueue on
macOS/BSD, select on Windows) so a single thread can handle multiple clients
without burning CPU when clients are idle.

Quick start
───────────

Echo server::

    from tcp_server import TcpServer

    server = TcpServer(host="127.0.0.1", port=6380)
    server.serve_forever()   # echoes every byte back to the sender

Shout server (uppercase everything)::

    from tcp_server import TcpServer

    server = TcpServer(
        host="127.0.0.1",
        port=6380,
        handler=lambda data: data.upper(),
    )
    server.serve_forever()

Context manager::

    import threading
    from tcp_server import TcpServer

    with TcpServer(host="127.0.0.1", port=6380) as server:
        t = threading.Thread(target=server.serve_forever, daemon=True)
        t.start()
        # ... do work ...
        server.stop()

Public API
──────────

``TcpServer``
    The main class. Accepts host, port, handler, backlog, buffer_size.

``Handler``
    Type alias: ``Callable[[bytes], bytes]``.

``__version__``
    Package version string.
"""

from tcp_server.tcp_server import Handler, TcpServer

__all__ = [
    "TcpServer",
    "Handler",
]

__version__ = "0.1.0"
