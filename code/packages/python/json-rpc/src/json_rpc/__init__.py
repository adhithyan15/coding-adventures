"""JSON-RPC 2.0 — framed transport and server dispatch over stdin/stdout.

This package implements the JSON-RPC 2.0 specification as defined at
https://www.jsonrpc.org/specification. It is the wire protocol beneath
the Language Server Protocol (LSP) and will be used by every LSP server
built in coding-adventures.

Overview
--------

JSON-RPC 2.0 is a lightweight remote procedure call protocol that encodes
procedure calls as JSON objects. An LSP session looks like this::

    Client (editor)                      Server (our LSP)
    ──────────────────────────────────────────────────────
    Request  {"method":"initialize"} ──► dispatched to handler
                                    ◄── Response {"result":{...}}
    Notification {"method":"didOpen"} ──► handler called, no reply
    Request  {"method":"hover"}  ──────► dispatched to handler
                                    ◄── Response {"result":{...}}
    EOF (editor exits)               ──► serve() returns

The four building blocks
------------------------

1. **Message types** — ``Request``, ``Response``, ``ResponseError``,
   ``Notification``. Plain dataclasses, easy to inspect in tests.

2. **MessageReader** — reads one Content-Length-framed message from a binary
   stream; returns ``None`` on EOF.

3. **MessageWriter** — writes one message to a binary stream with proper
   Content-Length framing.

4. **Server** — combines reader + writer with a method dispatch table.
   Register handlers with ``on_request`` and ``on_notification``, then call
   ``serve()`` to start the blocking loop.

Quick start
-----------

::

    import sys
    from json_rpc import Server, ResponseError, PARSE_ERROR

    server = (
        Server(sys.stdin.buffer, sys.stdout.buffer)
        .on_request("initialize", lambda id, params: {"capabilities": {}})
        .on_request("shutdown", lambda id, params: None)
        .on_notification("exit", lambda params: None)
    )
    server.serve()

Error codes
-----------

Standard error codes are available as module-level constants::

    from json_rpc import PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND
    from json_rpc import INVALID_PARAMS, INTERNAL_ERROR
"""

from json_rpc.errors import (
    INTERNAL_ERROR,
    INVALID_PARAMS,
    INVALID_REQUEST,
    METHOD_NOT_FOUND,
    PARSE_ERROR,
)
from json_rpc.message import (
    JsonRpcError,
    Message,
    Notification,
    Request,
    Response,
    ResponseError,
    message_to_dict,
    parse_message,
)
from json_rpc.reader import MessageReader
from json_rpc.server import Server
from json_rpc.writer import MessageWriter

__all__ = [
    # Error codes
    "PARSE_ERROR",
    "INVALID_REQUEST",
    "METHOD_NOT_FOUND",
    "INVALID_PARAMS",
    "INTERNAL_ERROR",
    # Message types
    "Request",
    "Response",
    "ResponseError",
    "Notification",
    "Message",
    "JsonRpcError",
    # Parsing helpers
    "parse_message",
    "message_to_dict",
    # I/O
    "MessageReader",
    "MessageWriter",
    # Server
    "Server",
]
