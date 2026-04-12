"""rpc — Codec-agnostic RPC primitive.

This package defines the abstract RPC layer that sits beneath codec-specific
packages like ``json-rpc``, ``msgpack-rpc``, and ``protobuf-rpc``.

The core idea is *separation of concerns*:

- **This package** handles method dispatch, id correlation, error handling,
  handler registration, and panic recovery.
- **The codec** (injected at construction time) handles serialisation —
  converting :class:`~rpc.message.RpcMessage` objects to/from bytes.
- **The framer** (injected at construction time) handles stream framing —
  splitting a raw byte stream into discrete chunks (frames).

Layer diagram::

    Application
        │
    RpcServer / RpcClient  ←── this package
        │
    RpcCodec               ←── e.g. coding-adventures-json-rpc
        │
    RpcFramer              ←── e.g. ContentLengthFramer
        │
    Transport (stdin/stdout, TCP, …)

Public API
----------

Message types::

    from rpc import RpcRequest, RpcResponse, RpcErrorResponse, RpcNotification
    from rpc import RpcId, RpcMessage

Server::

    from rpc import RpcServer
    server = RpcServer(codec, framer)
    server.on_request("add", lambda id, p: p["a"] + p["b"])
    server.on_notification("log", lambda p: print(p))
    server.serve()

Client::

    from rpc import RpcClient, RpcRemoteError
    client = RpcClient(codec, framer)
    result = client.request("add", {"a": 1, "b": 2})
    client.notify("log", {"msg": "hello"})

Error codes::

    from rpc import PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND
    from rpc import INVALID_PARAMS, INTERNAL_ERROR
    from rpc import RpcDecodeError

Protocol interfaces (for type-checking only)::

    from rpc import RpcCodec, RpcFramer
"""

from __future__ import annotations

from rpc.client import RpcClient, RpcRemoteError
from rpc.codec import RpcCodec
from rpc.errors import (
    INTERNAL_ERROR,
    INVALID_PARAMS,
    INVALID_REQUEST,
    METHOD_NOT_FOUND,
    PARSE_ERROR,
    RpcDecodeError,
)
from rpc.framer import RpcFramer
from rpc.message import (
    RpcErrorResponse,
    RpcId,
    RpcMessage,
    RpcNotification,
    RpcRequest,
    RpcResponse,
)
from rpc.server import RpcServer

__all__ = [
    # Message types
    "RpcRequest",
    "RpcResponse",
    "RpcErrorResponse",
    "RpcNotification",
    "RpcId",
    "RpcMessage",
    # Server and client
    "RpcServer",
    "RpcClient",
    "RpcRemoteError",
    # Protocol interfaces
    "RpcCodec",
    "RpcFramer",
    # Error codes
    "PARSE_ERROR",
    "INVALID_REQUEST",
    "METHOD_NOT_FOUND",
    "INVALID_PARAMS",
    "INTERNAL_ERROR",
    # Exceptions
    "RpcDecodeError",
]
