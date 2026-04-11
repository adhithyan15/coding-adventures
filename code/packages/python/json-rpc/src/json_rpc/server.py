"""JSON-RPC 2.0 Server — the read-dispatch-write loop.

The Server is the highest-level abstraction in this package. It wraps a
``MessageReader`` and a ``MessageWriter`` with a *method dispatch table* —
a dictionary mapping method names to handler functions.

Architecture
------------

                       ┌──────────────────────────────────┐
    stdin ──────────── │  MessageReader                   │
                       │    reads framed messages          │
                       └──────────────┬───────────────────┘
                                      │ Message
                                      ▼
                       ┌──────────────────────────────────┐
                       │  Server.serve() dispatch loop    │
                       │                                  │
                       │  Request?  → look up handler     │
                       │             call handler(id,params)│
                       │             write Response        │
                       │                                  │
                       │  Notification? → look up handler │
                       │                  call handler(params)│
                       │                  (no response!)  │
                       │                                  │
                       │  Unknown method? → write -32601  │
                       └──────────────┬───────────────────┘
                                      │ Response (if applicable)
                                      ▼
                       ┌──────────────────────────────────┐
                       │  MessageWriter                   │
                       │    writes framed responses       │
                       └──────────────────────────────────┘
                                      │
    stdout ─────────────────────────────

Why Single-Threaded?
---------------------

The ``serve()`` loop processes one message at a time (sequential). This
matches the LSP model: editors send requests one at a time and wait for
the response before sending the next (with the exception of notifications
and ``$/cancelRequest``).

A single-threaded server is:

- **Simple** — no locks, no race conditions, no shared state.
- **Correct** — for the LSP use case, it is the right model.
- **Predictable** — request N is always fully handled before request N+1.

If future language servers need concurrent request handling (e.g. for
``$/cancelRequest``), a thread-pool variant can be layered on top without
changing the handler API.

Handler Contract
-----------------

Request handlers::

    def handle_initialize(id, params):
        return {"capabilities": {}}  # success → return any JSON-serializable value

    def handle_broken(id, params):
        return ResponseError(code=-32602, message="bad params")  # failure → return ResponseError

Notification handlers::

    def handle_did_open(params):
        # fire and forget — no return value needed
        store_document(params["textDocument"])

The server checks the return type: if a request handler returns a
``ResponseError``, the server sends an error response. Otherwise it wraps
the return value in a success response.
"""

from __future__ import annotations

from typing import IO, Any, Callable

from json_rpc.errors import INTERNAL_ERROR, METHOD_NOT_FOUND
from json_rpc.message import (
    JsonRpcError,
    Message,
    Notification,
    Request,
    Response,
    ResponseError,
)
from json_rpc.reader import MessageReader
from json_rpc.writer import MessageWriter

# Type aliases for handler signatures
RequestHandler = Callable[[Any, Any], Any]
NotificationHandler = Callable[[Any], None]


class Server:
    """A JSON-RPC 2.0 server that drives the read-dispatch-write loop.

    Typical usage::

        server = Server(sys.stdin.buffer, sys.stdout.buffer)
        server.on_request("initialize", lambda id, params: {"capabilities": {}})
        server.on_notification("textDocument/didOpen", lambda params: None)
        server.serve()  # blocks until stdin closes

    The ``on_request`` and ``on_notification`` methods return ``self`` so
    calls can be chained::

        server = (
            Server(sys.stdin.buffer, sys.stdout.buffer)
            .on_request("foo", handle_foo)
            .on_notification("bar", handle_bar)
        )
        server.serve()

    Args:
        in_stream: Binary-mode readable stream (stdin in production).
        out_stream: Binary-mode writable stream (stdout in production).
    """

    def __init__(self, in_stream: IO[bytes], out_stream: IO[bytes]) -> None:
        self._reader = MessageReader(in_stream)
        self._writer = MessageWriter(out_stream)
        self._request_handlers: dict[str, RequestHandler] = {}
        self._notification_handlers: dict[str, NotificationHandler] = {}

    def on_request(self, method: str, handler: RequestHandler) -> "Server":
        """Register a handler for a JSON-RPC request method.

        The handler is called with ``(id, params)`` when a Request arrives
        with the matching ``method``. It must return either:

        - Any JSON-serializable value → sent as a success Response.
        - A ``ResponseError`` instance → sent as an error Response.

        Args:
            method: The method name to handle (e.g. ``"textDocument/hover"``).
            handler: A callable ``(id, params) → result | ResponseError``.

        Returns:
            ``self`` — allows method chaining.
        """
        self._request_handlers[method] = handler
        return self

    def on_notification(
        self, method: str, handler: NotificationHandler
    ) -> "Server":
        """Register a handler for a JSON-RPC notification method.

        The handler is called with ``(params,)`` when a Notification arrives
        with the matching ``method``. The return value is ignored — no
        response is ever sent for a notification.

        Args:
            method: The method name to handle (e.g. ``"textDocument/didOpen"``).
            handler: A callable ``(params) → None``.

        Returns:
            ``self`` — allows method chaining.
        """
        self._notification_handlers[method] = handler
        return self

    def serve(self) -> None:
        """Start the blocking read-dispatch-write loop.

        Reads messages until EOF (when the client closes stdin). For each message:

        - **Request** → look up handler by ``method``. Call it and send the
          Response. If no handler: send ``-32601 Method not found``.
        - **Notification** → look up handler by ``method``. Call it silently.
          If no handler: do nothing (spec requires silence for unknown notifications).
        - **Response** → ignored (servers don't handle incoming responses in
          this single-server model; a future client-side API would forward to
          a pending-request table).

        Exceptions from handlers are caught and converted to ``-32603 Internal error``
        responses to keep the server alive despite buggy handlers.
        """
        while True:
            try:
                msg = self._reader.read_message()
            except JsonRpcError as exc:
                # Framing or parse error — send an error response with id=None
                # because we could not determine the request id.
                self._send_error(None, exc.code, exc.message)
                continue

            if msg is None:
                # Clean EOF — client disconnected.
                break

            self._dispatch(msg)

    # ------------------------------------------------------------------
    # Private helpers
    # ------------------------------------------------------------------

    def _dispatch(self, msg: Message) -> None:
        """Route a parsed message to the appropriate handler."""
        if isinstance(msg, Request):
            self._handle_request(msg)
        elif isinstance(msg, Notification):
            self._handle_notification(msg)
        # Responses are silently ignored in this server model.

    def _handle_request(self, req: Request) -> None:
        """Invoke the registered handler for a Request and send the Response."""
        handler = self._request_handlers.get(req.method)

        if handler is None:
            # Spec §5.1: unknown method → -32601 Method not found.
            self._send_error(req.id, METHOD_NOT_FOUND, "Method not found")
            return

        # Call the handler, catching any unhandled exceptions.
        try:
            result = handler(req.id, req.params)
        except Exception as exc:  # noqa: BLE001
            self._send_error(req.id, INTERNAL_ERROR, f"Internal error: {exc}")
            return

        # If the handler returned a ResponseError, send an error response.
        if isinstance(result, ResponseError):
            response = Response(id=req.id, error=result)
        else:
            # Any other return value is a success result.
            response = Response(id=req.id, result=result)

        self._writer.write_message(response)

    def _handle_notification(self, notif: Notification) -> None:
        """Invoke the registered handler for a Notification (no response sent)."""
        handler = self._notification_handlers.get(notif.method)
        if handler is None:
            # Per spec: silently ignore unknown notifications.
            return

        try:
            handler(notif.params)
        except Exception:  # noqa: BLE001
            # Notification handlers must not generate error responses even
            # if they raise. The spec prohibits error responses to notifications.
            pass

    def _send_error(
        self, msg_id: Any, code: int, message: str, data: Any = None
    ) -> None:
        """Write an error Response to the output stream."""
        error = ResponseError(code=code, message=message, data=data)
        response = Response(id=msg_id, error=error)
        self._writer.write_message(response)
