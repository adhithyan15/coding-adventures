"""RPC Message Types — the codec-agnostic envelope for procedure calls.

Overview
--------

Every RPC interaction is one of four message types:

1. **RpcRequest** — "Please do X and tell me the result."
   Carries an ``id`` so the response can be matched back to this request.

2. **RpcResponse** — "Here is the result of request #id."
   Carries the same ``id`` as the request that triggered it.

3. **RpcErrorResponse** — "Request #id failed.  Here is why."
   Carries the same ``id`` as the request (or ``None`` if the request was
   so malformed that its id could not be read).

4. **RpcNotification** — "FYI: event Y happened."
   Has no ``id``.  The sender does not expect a reply.

All four types carry a type parameter ``V`` — the *value* type.  ``V`` is
whatever the codec uses as its native dynamic-value type:

- JSON codec → ``V = dict | list | str | int | float | bool | None``
- MessagePack codec → ``V = msgpack value``
- Protobuf codec → ``V = proto.Message``
- Test code → ``V = Any``

Because Python is dynamically typed, ``V`` is expressed as a ``TypeVar`` bound
by ``Generic`` — the runtime does not enforce it, but mypy / pyright will.

RpcId
-----

Request ids are either strings or integers.  ``None`` is permitted only for
:class:`RpcErrorResponse` when the original request was so malformed that its
id could not be extracted from the framed bytes.

.. code-block:: text

    RpcId = str | int

Message taxonomy
-----------------

.. code-block:: text

    RpcMessage<V>
    ├── RpcRequest<V>       { id: RpcId, method: str, params: V | None }
    ├── RpcResponse<V>      { id: RpcId, result: V | None }
    ├── RpcErrorResponse<V> { code: int, message: str, id: RpcId | None, data: V | None }
    └── RpcNotification<V>  { method: str, params: V | None }

Dataclass design
-----------------

All four message types are plain ``@dataclass`` objects.  Using dataclasses
gives us:

- Free ``__eq__`` (needed for ``assert msg == expected`` in tests).
- Free ``__repr__`` (useful for debugging and error messages).
- Clear constructor with named parameters.

Example::

    req = RpcRequest(id=1, method="add", params={"a": 1, "b": 2})
    resp = RpcResponse(id=1, result=3)
    err  = RpcErrorResponse(code=-32601, message="Method not found", id=1)
    notif = RpcNotification(method="ping", params=None)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, Optional, TypeVar, Union

# ---------------------------------------------------------------------------
# Type variables
# ---------------------------------------------------------------------------

V = TypeVar("V")
"""The codec's native value type.

In JSON-based codecs this would typically be ``Any`` (dict, list, str, int, …).
In a typed Protobuf codec it would be a concrete ``proto.Message`` subtype.
In tests it is usually ``Any``.
"""

# ---------------------------------------------------------------------------
# RpcId
# ---------------------------------------------------------------------------

RpcId = Union[str, int]
"""Type alias for RPC message identifiers.

An id uniquely correlates a request to its response.  The RPC specification
(following JSON-RPC 2.0) restricts ids to strings and integers.

``None`` is *not* a valid ``RpcId`` for requests — it is permitted only in
:class:`RpcErrorResponse` to signal "the request was so malformed we could
not read its id".
"""


# ---------------------------------------------------------------------------
# Message dataclasses
# ---------------------------------------------------------------------------


@dataclass
class RpcRequest(Generic[V]):
    """A request from client to server.

    The client sends a Request when it wants the server to execute a named
    procedure and return a result.  The ``id`` field links this Request to
    the eventual :class:`RpcResponse` or :class:`RpcErrorResponse`.

    Attributes:
        id: Unique identifier for this request.  Used by the client to match
            the response.  Must be a ``str`` or ``int``.
        method: The name of the procedure to call, e.g. ``"add"`` or
            ``"textDocument/hover"``.
        params: Optional parameters to pass to the procedure.  The shape of
            ``params`` is defined by the method — it could be a dict, a list,
            a number, or ``None``.

    Example::

        req = RpcRequest(id=42, method="sqrt", params={"n": 144})
        # → server should return 12.0
    """

    id: RpcId
    method: str
    params: Optional[V] = field(default=None)


@dataclass
class RpcResponse(Generic[V]):
    """A successful response from server to client.

    Sent by the server when a request handler completes without error.
    Carries the ``id`` of the original request so the client can match them.

    Attributes:
        id: The id of the :class:`RpcRequest` this is responding to.
        result: The return value of the handler.  May be ``None`` for handlers
            that perform side-effects and return nothing meaningful.

    Example::

        resp = RpcResponse(id=42, result=12.0)
        # → sqrt(144) = 12.0
    """

    id: RpcId
    result: Optional[V] = field(default=None)


@dataclass
class RpcErrorResponse(Generic[V]):
    """An error response from server to client.

    Sent by the server when:

    - A request handler raises an exception → :data:`~rpc.errors.INTERNAL_ERROR`
    - The method is not registered → :data:`~rpc.errors.METHOD_NOT_FOUND`
    - The codec could not decode the frame → :data:`~rpc.errors.PARSE_ERROR`
    - The decoded object is not a valid RPC message → :data:`~rpc.errors.INVALID_REQUEST`
    - The handler explicitly signals a validation error → :data:`~rpc.errors.INVALID_PARAMS`

    Attributes:
        code: Integer error code.  Use the constants in :mod:`rpc.errors`.
        message: Human-readable explanation of the error.
        id: The id of the failing :class:`RpcRequest`, or ``None`` if the
            request was so malformed its id could not be read.
        data: Optional extra context (e.g. a stack trace string, extra fields).

    Example::

        err = RpcErrorResponse(
            code=-32601,
            message="Method not found",
            id=42,
        )
    """

    code: int
    message: str
    id: Optional[RpcId] = field(default=None)
    data: Optional[V] = field(default=None)


@dataclass
class RpcNotification(Generic[V]):
    """A fire-and-forget notification from client or server.

    Notifications have no ``id`` and generate no response.  They are used for
    events where the sender does not need confirmation:

    - Client → Server: "I opened a file" (LSP ``textDocument/didOpen``)
    - Server → Client: "Here are updated diagnostics" (LSP ``publishDiagnostics``)

    Attributes:
        method: The name of the event or procedure.
        params: Optional parameters describing the event.

    Example::

        notif = RpcNotification(method="log", params={"level": "info", "msg": "ready"})
    """

    method: str
    params: Optional[V] = field(default=None)


# ---------------------------------------------------------------------------
# Union type alias for dispatch
# ---------------------------------------------------------------------------

RpcMessage = Union[
    RpcRequest[V],
    RpcResponse[V],
    RpcErrorResponse[V],
    RpcNotification[V],
]
"""Union of all four message types.

Use this as a type hint when a function accepts or returns any kind of RPC
message::

    def dispatch(msg: RpcMessage) -> None:
        if isinstance(msg, RpcRequest):
            ...
        elif isinstance(msg, RpcNotification):
            ...
"""
