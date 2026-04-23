"""RpcServer — the codec-agnostic read-dispatch-write loop.

Overview
--------

The ``RpcServer`` is the highest-level abstraction in this package.  It wires
together an :class:`~rpc.codec.RpcCodec` and an :class:`~rpc.framer.RpcFramer`
with a *method dispatch table* — a dictionary that maps method names to handler
functions.

The server is deliberately *not* JSON-aware, *not* Content-Length-aware, and
*not* transport-aware.  Those concerns are delegated to the codec and framer
respectively.

Architecture
------------

.. code-block:: text

    Transport byte stream
         │
         ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcFramer.read_frame()                                    │
    │  Reads the next discrete chunk of bytes from the stream.   │
    └─────────────────────────────┬──────────────────────────────┘
                                  │ raw bytes
                                  ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcCodec.decode(bytes)                                    │
    │  Deserialises bytes → RpcMessage.                          │
    │  Raises RpcDecodeError on failure.                         │
    └─────────────────────────────┬──────────────────────────────┘
                                  │ RpcMessage
                                  ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcServer._dispatch(msg)                                  │
    │                                                            │
    │  RpcRequest?     → look up handler                         │
    │                    call handler(id, params)                │
    │                    write RpcResponse or RpcErrorResponse   │
    │                                                            │
    │  RpcNotification?→ look up handler                         │
    │                    call handler(params) — no response!     │
    │                                                            │
    │  Unknown method? → write -32601 error response            │
    │  Handler raises? → write -32603 error response            │
    └─────────────────────────────┬──────────────────────────────┘
                                  │ RpcMessage (response)
                                  ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcCodec.encode(msg)                                      │
    │  Serialises response RpcMessage → bytes.                   │
    └─────────────────────────────┬──────────────────────────────┘
                                  │ raw bytes
                                  ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcFramer.write_frame(bytes)                              │
    │  Wraps bytes in framing envelope and writes to transport.  │
    └────────────────────────────────────────────────────────────┘
         │
         ▼
    Transport byte stream

Why Single-Threaded?
---------------------

The ``serve()`` loop processes one message at a time (sequential).  This
matches the LSP model: editors send requests and wait for responses before
sending the next (with the exception of notifications and
``$/cancelRequest``).

A single-threaded server is:

- **Simple** — no locks, no race conditions, no shared state.
- **Correct** — for the LSP use case, sequential processing is the right model.
- **Predictable** — request N is always fully handled before request N+1.

If future servers need concurrent request handling (e.g., ``$/cancelRequest``),
a thread-pool variant can be layered on top without changing the handler API.

Handler Contract
-----------------

Request handlers receive ``(id, params)`` and return a value that becomes the
``result`` of the response::

    def handle_add(id, params):
        return params["a"] + params["b"]   # success → return the result

    def handle_broken(id, params):
        raise ValueError("bad params")     # failure → server catches and wraps in -32603

Notification handlers receive ``(params,)`` and return nothing::

    def handle_log(params):
        print(params["message"])           # fire and forget

Panic safety
-------------

Any exception raised by a handler — including ``SystemExit``, ``KeyboardInterrupt``,
and ``BaseException`` subclasses — is caught by the ``serve()`` loop.
This prevents a single bad handler from killing the server process.

A caught exception produces a ``-32603 Internal error`` response (for
requests) or is silently discarded (for notifications, which must never
generate responses per the spec).
"""

from __future__ import annotations

from typing import Any, Callable, Generic, Optional, TypeVar

from rpc.codec import RpcCodec
from rpc.errors import INTERNAL_ERROR, METHOD_NOT_FOUND, RpcDecodeError
from rpc.framer import RpcFramer
from rpc.message import (
    RpcErrorResponse,
    RpcId,
    RpcNotification,
    RpcRequest,
    RpcResponse,
)

V = TypeVar("V")

# Type aliases for handler signatures.
# A request handler receives (id, params) and returns a result value.
# A notification handler receives (params,) and returns nothing.
RequestHandler = Callable[[RpcId, Optional[Any]], Any]
NotificationHandler = Callable[[Optional[Any]], None]


class RpcServer(Generic[V]):
    """A codec-agnostic RPC server that drives the read-dispatch-write loop.

    The server owns two dispatch tables:

    - ``_request_handlers``: ``{method_name: fn(id, params) → result}``
    - ``_notification_handlers``: ``{method_name: fn(params) → None}``

    Typical usage::

        from rpc import RpcServer
        from my_codec import MyCodec
        from my_framer import MyFramer

        server = RpcServer(MyCodec(), MyFramer(stream))
        server.on_request("add", lambda id, p: p["a"] + p["b"])
        server.on_notification("log", lambda p: print(p["msg"]))
        server.serve()   # blocks until EOF

    Method chaining::

        server = (
            RpcServer(codec, framer)
            .on_request("initialize", handle_initialize)
            .on_notification("exit", handle_exit)
        )
        server.serve()

    Args:
        codec: Any object satisfying :class:`~rpc.codec.RpcCodec` — encodes
            and decodes :class:`~rpc.message.RpcMessage` objects.
        framer: Any object satisfying :class:`~rpc.framer.RpcFramer` — reads
            and writes discrete byte frames from/to the transport.
    """

    def __init__(self, codec: RpcCodec[V], framer: RpcFramer) -> None:
        self._codec = codec
        self._framer = framer
        self._request_handlers: dict[str, RequestHandler] = {}
        self._notification_handlers: dict[str, NotificationHandler] = {}

    def on_request(
        self,
        method: str,
        handler: Callable[[RpcId, Optional[V]], V],
    ) -> "RpcServer[V]":
        """Register a handler for a named request method.

        Calling ``on_request`` with the same method name twice replaces the
        earlier handler — the last registration wins.

        The handler is called with ``(id, params)`` when a request arrives.
        It should return the result value (any codec-serializable object).
        If the handler raises, the server catches the exception and sends a
        ``-32603 Internal error`` response.

        Args:
            method: The method name to handle, e.g. ``"add"`` or
                ``"textDocument/hover"``.
            handler: A callable ``(id: RpcId, params: V | None) → V``.

        Returns:
            ``self`` — allows method chaining.

        Example::

            server.on_request("greet", lambda id, p: f"Hello, {p['name']}!")
        """
        self._request_handlers[method] = handler  # type: ignore[assignment]
        return self

    def on_notification(
        self,
        method: str,
        handler: Callable[[Optional[V]], None],
    ) -> "RpcServer[V]":
        """Register a handler for a named notification method.

        Calling ``on_notification`` with the same method name twice replaces
        the earlier handler.

        The handler is called with ``(params,)`` when a notification arrives.
        Its return value is ignored.  If it raises, the exception is silently
        discarded — the spec prohibits sending error responses to notifications.

        Unknown notifications (no registered handler) are silently dropped.

        Args:
            method: The notification method name to handle.
            handler: A callable ``(params: V | None) → None``.

        Returns:
            ``self`` — allows method chaining.

        Example::

            server.on_notification("ping", lambda p: print("got ping"))
        """
        self._notification_handlers[method] = handler  # type: ignore[assignment]
        return self

    def serve(self) -> None:
        """Start the blocking read-dispatch-write loop.

        Reads frames from the framer until EOF.  For each frame:

        1. ``codec.decode(bytes)`` → :class:`~rpc.message.RpcMessage`.
           On :class:`~rpc.errors.RpcDecodeError`: send error response
           with ``id=None`` and continue (the connection stays alive).

        2. Dispatch:
           - :class:`~rpc.message.RpcRequest` → look up handler.
             If found: call it and send :class:`~rpc.message.RpcResponse`.
             If not found: send ``-32601 Method not found``.
             If handler raises: send ``-32603 Internal error``.
           - :class:`~rpc.message.RpcNotification` → look up handler.
             If found: call it (no response).
             If not found: silently drop (no response, no error).
           - :class:`~rpc.message.RpcResponse` / :class:`~rpc.message.RpcErrorResponse`:
             ignored (servers that only respond don't handle incoming responses).

        3. Repeat until ``framer.read_frame()`` returns ``None`` (clean EOF).

        Panic safety: handlers are wrapped in ``try/except BaseException`` so
        a handler that calls ``sys.exit()`` or raises ``KeyboardInterrupt``
        still produces an error response rather than killing the server.
        """
        while True:
            try:
                data = self._framer.read_frame()
            except Exception as exc:  # noqa: BLE001
                # Framing error — we cannot continue reading; break the loop.
                # This is an unrecoverable transport failure.
                raise exc

            if data is None:
                # Clean EOF — the remote side closed the connection.
                break

            # Attempt to decode the frame.  A codec failure produces an error
            # response with id=None (we don't know the request id).
            try:
                msg = self._codec.decode(data)
            except RpcDecodeError as exc:
                self._send_error(None, exc.code, exc.message)
                continue

            self._dispatch(msg)

    # ------------------------------------------------------------------
    # Private helpers
    # ------------------------------------------------------------------

    def _dispatch(self, msg: Any) -> None:
        """Route a parsed message to the appropriate handler."""
        if isinstance(msg, RpcRequest):
            self._handle_request(msg)
        elif isinstance(msg, RpcNotification):
            self._handle_notification(msg)
        # RpcResponse / RpcErrorResponse arriving at a server are ignored
        # in the simple server model.  A bidirectional peer would forward
        # them to a pending-request table.

    def _handle_request(self, req: RpcRequest[V]) -> None:
        """Invoke the registered handler for a request and send the response."""
        handler = self._request_handlers.get(req.method)

        if handler is None:
            # Spec §5.1 (inherited from JSON-RPC): unknown method → -32601.
            self._send_error(req.id, METHOD_NOT_FOUND, "Method not found")
            return

        # Wrap the handler call in a broad except to implement panic safety.
        # We catch BaseException (not just Exception) to handle SystemExit,
        # KeyboardInterrupt, and other BaseException subclasses that could
        # otherwise silently kill the server.
        try:
            result = handler(req.id, req.params)
        except BaseException as exc:  # noqa: BLE001
            self._send_error(req.id, INTERNAL_ERROR, f"Internal error: {exc}")
            return

        # Build and send a successful response.
        response = RpcResponse(id=req.id, result=result)
        self._write_msg(response)

    def _handle_notification(self, notif: RpcNotification[V]) -> None:
        """Invoke the registered handler for a notification.  Never sends a response."""
        handler = self._notification_handlers.get(notif.method)
        if handler is None:
            # Per spec: silently ignore unknown notifications.
            return

        try:
            handler(notif.params)
        except BaseException:  # noqa: BLE001
            # Notification handlers must not generate error responses even if
            # they raise.  The spec prohibits error responses to notifications.
            pass

    def _send_error(
        self,
        msg_id: Optional[RpcId],
        code: int,
        message: str,
        data: Optional[V] = None,
    ) -> None:
        """Encode and write an :class:`~rpc.message.RpcErrorResponse`."""
        err = RpcErrorResponse(code=code, message=message, id=msg_id, data=data)
        self._write_msg(err)

    def _write_msg(self, msg: Any) -> None:
        """Encode *msg* with the codec and write the frame."""
        encoded = self._codec.encode(msg)
        self._framer.write_frame(encoded)
