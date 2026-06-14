"""coding_adventures.irc_server_native — a high-performance IRC server for Python.

All IRC and TCP logic runs in Rust (the ``irc-net-reactor`` engine on the
home-grown kqueue/epoll reactor).  Python only *launches and controls* the
server — there is no per-message callback into Python, so the API is tiny:

    from coding_adventures.irc_server_native import IrcServer

    server = IrcServer(host="127.0.0.1", port=6667)
    server.serve()   # blocks (GIL released) until server.stop()

See :class:`IrcServer` for the full control surface.
"""

from .server import IrcServer

__all__ = ["IrcServer"]
__version__ = "0.1.0"
