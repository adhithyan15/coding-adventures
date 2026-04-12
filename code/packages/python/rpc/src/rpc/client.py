"""RpcClient — the codec-agnostic RPC client.

Overview
--------

The ``RpcClient`` sends requests to a remote server and waits (blocking) for
matching responses.  It also sends fire-and-forget notifications.

Think of the client like a telephone caller:

- ``request()`` is like placing a call and waiting on hold until the other
  party picks up and answers.
- ``notify()`` is like leaving a voicemail — you speak your piece and hang up
  without waiting for a reply.

Architecture
------------

.. code-block:: text

    Application code
         │  request("add", params)
         │  notify("log", params)
         ▼
    ┌────────────────────────────────────────────────────────────┐
    │  RpcClient                                                 │
    │  · auto-generates request ids (1, 2, 3, …)                │
    │  · encodes with codec, writes with framer                  │
    │  · blocks in read loop until matching response arrives     │
    │  · dispatches notifications to registered handlers         │
    └────────────────────────────────────────────────────────────┘
         │  codec.encode / framer.write_frame
         │  framer.read_frame / codec.decode
         ▼
    Transport (stdin/stdout, TCP, BytesIO, …)

Id Management
--------------

The client maintains a monotonically increasing integer counter starting at 1.
Each call to ``request()`` increments the counter and uses the new value as
the request id.

.. code-block:: text

    next_id = 1
    request("foo") → uses id=1, next_id becomes 2
    request("bar") → uses id=2, next_id becomes 3
    notify("log")  → no id consumed
    request("baz") → uses id=3, next_id becomes 4

Blocking Request Flow
----------------------

.. code-block:: text

    request(method, params):
      id = next_id++
      frame = codec.encode(RpcRequest(id, method, params))
      framer.write_frame(frame)

      loop:
        data = framer.read_frame()
        if data is None → raise ConnectionError("connection closed")
        msg = codec.decode(data)

        if isinstance(msg, (RpcResponse, RpcErrorResponse)) and msg.id == id:
            if isinstance(msg, RpcErrorResponse):
                raise RpcRemoteError(msg)
            return msg.result

        if isinstance(msg, RpcNotification):
            dispatch to notification handler if registered
            continue waiting — this was a server push, not our response

        # Any other message (response for different id, etc.) → ignore.

Notification Handlers
----------------------

The client can register handlers for server-initiated notifications (server
push).  These arrive while the client is blocked inside ``request()`` waiting
for a response::

    client.on_notification("diagnostics", lambda p: handle_diags(p))

This is optional — if no handler is registered for a notification method, the
notification is silently discarded.

Error Handling
---------------

``request()`` raises :class:`RpcRemoteError` when the server responds with an
:class:`~rpc.message.RpcErrorResponse`.  The exception carries the full error
response so callers can inspect the ``code``, ``message``, and ``data``.

Example::

    try:
        result = client.request("divide", {"a": 1, "b": 0})
    except RpcRemoteError as exc:
        print(f"Server error {exc.error.code}: {exc.error.message}")
"""

from __future__ import annotations

from typing import Any, Callable, Generic, Optional, TypeVar

from rpc.codec import RpcCodec
from rpc.errors import RpcDecodeError
from rpc.framer import RpcFramer
from rpc.message import (
    RpcErrorResponse,
    RpcId,
    RpcNotification,
    RpcRequest,
    RpcResponse,
)

V = TypeVar("V")

NotificationHandler = Callable[[Optional[Any]], None]


class RpcRemoteError(Exception):
    """Raised by :meth:`RpcClient.request` when the server returns an error.

    Wraps the :class:`~rpc.message.RpcErrorResponse` from the server so the
    caller can inspect the error code, message, and optional data.

    Attributes:
        error: The :class:`~rpc.message.RpcErrorResponse` received from the
            server.

    Example::

        try:
            result = client.request("divide", {"a": 1, "b": 0})
        except RpcRemoteError as exc:
            print(exc.error.code)      # -32602
            print(exc.error.message)   # "division by zero"
    """

    def __init__(self, error: RpcErrorResponse) -> None:  # type: ignore[type-arg]
        super().__init__(f"RPC error {error.code}: {error.message}")
        self.error = error

    def __repr__(self) -> str:
        return f"RpcRemoteError(code={self.error.code!r}, message={self.error.message!r})"


class RpcClient(Generic[V]):
    """A codec-agnostic RPC client that sends requests and receives responses.

    The client sends requests to a remote server (blocking until the matching
    response arrives) and fire-and-forget notifications.

    Typical usage::

        from rpc import RpcClient, RpcRemoteError
        from my_codec import MyCodec
        from my_framer import MyFramer

        client = RpcClient(MyCodec(), MyFramer(stream))
        try:
            result = client.request("add", {"a": 3, "b": 4})
            print(result)  # 7
        except RpcRemoteError as exc:
            print(f"Error: {exc.error.message}")

        client.notify("log", {"msg": "done"})

    Method chaining for notification handlers::

        client = (
            RpcClient(codec, framer)
            .on_notification("diagnostics", handle_diags)
            .on_notification("progress", handle_progress)
        )

    Args:
        codec: Any object satisfying :class:`~rpc.codec.RpcCodec`.
        framer: Any object satisfying :class:`~rpc.framer.RpcFramer`.
    """

    def __init__(self, codec: RpcCodec[V], framer: RpcFramer) -> None:
        self._codec = codec
        self._framer = framer
        self._next_id: int = 1
        self._notification_handlers: dict[str, NotificationHandler] = {}

    def on_notification(
        self,
        method: str,
        handler: Callable[[Optional[V]], None],
    ) -> "RpcClient[V]":
        """Register a handler for server-initiated notifications.

        The handler is called when a :class:`~rpc.message.RpcNotification`
        with the matching ``method`` arrives while the client is blocked inside
        :meth:`request`.

        Args:
            method: The notification method name to handle.
            handler: A callable ``(params: V | None) → None``.

        Returns:
            ``self`` — allows method chaining.

        Example::

            client.on_notification("ping", lambda p: print("server pinged us"))
        """
        self._notification_handlers[method] = handler  # type: ignore[assignment]
        return self

    def request(
        self,
        method: str,
        params: Optional[V] = None,
    ) -> V:
        """Send a request and block until the matching response arrives.

        Generates a unique request id, encodes the :class:`~rpc.message.RpcRequest`,
        writes it via the framer, then reads frames in a loop until the
        matching response (same ``id``) is received.

        Notifications that arrive while waiting are dispatched to any
        registered handlers (see :meth:`on_notification`) and then the loop
        continues waiting.

        Args:
            method: The method name to call on the server.
            params: Optional parameters to pass.  Type must be serializable by
                the codec.

        Returns:
            The ``result`` value from the server's
            :class:`~rpc.message.RpcResponse`.

        Raises:
            :class:`RpcRemoteError`: If the server responds with an
                :class:`~rpc.message.RpcErrorResponse`.
            ``ConnectionError``: If the connection is closed before the
                matching response arrives.
            :class:`~rpc.errors.RpcDecodeError`: If the codec cannot decode
                a response frame.

        Example::

            result = client.request("multiply", {"a": 6, "b": 7})
            # result == 42
        """
        # Allocate a fresh id for this request and increment the counter.
        req_id: RpcId = self._next_id
        self._next_id += 1

        # Build the request message and send it.
        req = RpcRequest(id=req_id, method=method, params=params)
        self._framer.write_frame(self._codec.encode(req))

        # Read frames until we find the one with our id.
        while True:
            data = self._framer.read_frame()

            if data is None:
                # Remote side closed the connection before responding.
                raise ConnectionError(
                    f"Connection closed before response to request id={req_id}"
                )

            msg = self._codec.decode(data)

            if isinstance(msg, RpcResponse) and msg.id == req_id:
                # Success — return the result value.
                return msg.result  # type: ignore[return-value]

            if isinstance(msg, RpcErrorResponse) and msg.id == req_id:
                # The server signalled an error for our request.
                raise RpcRemoteError(msg)

            if isinstance(msg, RpcNotification):
                # Server-push notification received while waiting.
                # Dispatch to the registered handler (if any) and keep waiting.
                handler = self._notification_handlers.get(msg.method)
                if handler is not None:
                    try:
                        handler(msg.params)
                    except Exception:  # noqa: BLE001
                        # Don't let a notification handler crash the wait loop.
                        pass
                continue

            # Any other message (response for different id, unexpected type)
            # is ignored — keep reading.

    def notify(
        self,
        method: str,
        params: Optional[V] = None,
    ) -> None:
        """Send a fire-and-forget notification.

        Encodes the :class:`~rpc.message.RpcNotification` and writes it via
        the framer.  Returns immediately without waiting for any response
        (notifications never generate responses by spec).

        Args:
            method: The notification method name.
            params: Optional parameters.

        Example::

            client.notify("textDocument/didSave", {"uri": "file:///foo.py"})
        """
        notif = RpcNotification(method=method, params=params)
        self._framer.write_frame(self._codec.encode(notif))
