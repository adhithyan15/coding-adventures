"""JSON-RPC 2.0 Message Types.

JSON-RPC 2.0 defines four message shapes that flow between a client (e.g., a
code editor) and a server (e.g., our Brainfuck LSP). All messages share a
single structural marker: ``"jsonrpc": "2.0"``.

The Four Message Types
-----------------------

Think of a JSON-RPC session like a restaurant:

- **Request** — the customer places an order (has an ``id`` so the waiter can
  bring it back to the right table, and a ``method`` naming the dish).
- **Response** — the kitchen sends the food back (``id`` matches the order,
  ``result`` is the food or ``error`` is what went wrong).
- **Notification** — a broadcast announcement ("kitchen is closing in 5 min")
  — no ``id``, no reply expected.
- **ResponseError** — the error envelope inside a failed Response.

Discriminating Requests from Notifications
-------------------------------------------

There is no explicit ``type`` field in the JSON-RPC wire format. Instead:

    - A message with ``"id"`` is a **Request** (client wants a response).
    - A message without ``"id"`` is a **Notification** (client does not wait).
    - A message with ``"result"`` or ``"error"`` (and an ``"id"``) is a **Response**.

This is why ``parse_message`` inspects the key set rather than a discriminator.

Wire format examples
--------------------

Request::

    {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}

Response (success)::

    {"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}

Response (error)::

    {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}

Notification::

    {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///a.bf"}}
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any

from json_rpc.errors import INVALID_REQUEST, PARSE_ERROR


# ---------------------------------------------------------------------------
# ResponseError — the error envelope
# ---------------------------------------------------------------------------


@dataclass
class ResponseError:
    """An error object embedded inside a failed Response.

    Carries a numeric ``code`` (from the standard error code table or a
    server-defined range), a human-readable ``message``, and an optional
    ``data`` field for additional context.

    Example::

        ResponseError(
            code=-32601,
            message="Method not found",
            data="textDocument/hover is not registered",
        )
    """

    code: int
    message: str
    data: Any = field(default=None)


# ---------------------------------------------------------------------------
# Request — a call expecting a response
# ---------------------------------------------------------------------------


@dataclass
class Request:
    """A JSON-RPC 2.0 request from client to server.

    The ``id`` ties the response back to this request. The server must
    reply with a Response carrying the same ``id``.

    Attributes:
        id: Unique identifier for this request. String or integer; never None.
        method: The procedure name (e.g. ``"textDocument/hover"``).
        params: Optional arguments — dict, list, or None.

    Wire form::

        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
    """

    id: str | int
    method: str
    params: Any = field(default=None)


# ---------------------------------------------------------------------------
# Response — a reply to a Request
# ---------------------------------------------------------------------------


@dataclass
class Response:
    """A JSON-RPC 2.0 response sent by the server after handling a Request.

    Exactly one of ``result`` or ``error`` must be set (not both). If the
    handler succeeded, set ``result`` to any JSON-serializable value. If it
    failed, set ``error`` to a ``ResponseError``.

    The ``id`` must match the originating Request's ``id``. If the Request
    was unparseable, ``id`` may be ``None``.

    Attributes:
        id: Matches the Request's ``id``. ``None`` only if the request was
            so malformed that its ``id`` could not be recovered.
        result: The procedure's return value (present on success).
        error: The error envelope (present on failure).

    Wire form (success)::

        {"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}

    Wire form (error)::

        {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
    """

    id: str | int | None
    result: Any = field(default=None)
    error: ResponseError | None = field(default=None)


# ---------------------------------------------------------------------------
# Notification — a fire-and-forget message
# ---------------------------------------------------------------------------


@dataclass
class Notification:
    """A JSON-RPC 2.0 notification sent by the client (no response expected).

    Notifications have no ``id`` field. The server must silently ignore
    unknown notification methods — generating an error response would
    violate the spec.

    Attributes:
        method: The event name (e.g. ``"textDocument/didOpen"``).
        params: Optional arguments — dict, list, or None.

    Wire form::

        {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"..."}}
    """

    method: str
    params: Any = field(default=None)


# ---------------------------------------------------------------------------
# Union type alias
# ---------------------------------------------------------------------------

#: A JSON-RPC message is one of the three inbound types.
#: (ResponseError is not a top-level message — it lives inside Response.)
Message = Request | Response | Notification


# ---------------------------------------------------------------------------
# Parsing helpers
# ---------------------------------------------------------------------------


class JsonRpcError(Exception):
    """Raised when a framed message cannot be parsed into a valid Message.

    Carries an error ``code`` (from the standard table) so the caller can
    construct a proper JSON-RPC error Response.

    Attributes:
        code: A standard JSON-RPC error code (PARSE_ERROR or INVALID_REQUEST).
        message: A human-readable description of the failure.
    """

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def parse_message(raw: str) -> Message:
    """Parse a raw JSON string into a typed Message.

    This function is the entry point from the wire into the type system.
    It applies two layers of validation:

    1. **JSON parsing** — if ``raw`` is not valid JSON, raise
       ``JsonRpcError(PARSE_ERROR)``.
    2. **Schema validation** — if the JSON is not a valid JSON-RPC message
       shape, raise ``JsonRpcError(INVALID_REQUEST)``.

    Discrimination logic::

        Has "result" or "error" key?  →  Response
        Has "id" key?                 →  Request
        Otherwise                     →  Notification

    Args:
        raw: A UTF-8 JSON string (the payload after Content-Length framing).

    Returns:
        A ``Request``, ``Response``, or ``Notification`` instance.

    Raises:
        JsonRpcError: With ``PARSE_ERROR`` if JSON is invalid.
        JsonRpcError: With ``INVALID_REQUEST`` if JSON is valid but not a
            well-formed JSON-RPC message.
    """
    # Step 1: Parse JSON. Any exception here means the bytes were not valid JSON.
    try:
        obj = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise JsonRpcError(
            PARSE_ERROR, f"Parse error: {exc}"
        ) from exc

    # Step 2: Must be a JSON object (dict), not an array or scalar.
    if not isinstance(obj, dict):
        raise JsonRpcError(
            INVALID_REQUEST,
            "Invalid Request: top-level value must be a JSON object",
        )

    # Step 3: The "jsonrpc" field must be exactly "2.0".
    # This is a cheap guard against accidentally connecting a JSON-RPC 1.0
    # client or a completely different protocol.
    if obj.get("jsonrpc") != "2.0":
        raise JsonRpcError(
            INVALID_REQUEST,
            'Invalid Request: missing or wrong "jsonrpc" field (expected "2.0")',
        )

    # Step 4: Discriminate on key presence.
    # The JSON-RPC spec uses structural typing, not a "type" discriminator.

    # Responses are identified by having "result" or "error" in the object.
    # They also have "id", but so do Requests — so we check result/error first.
    if "result" in obj or "error" in obj:
        # Parse as Response. "id" may be null if the request was unparseable.
        msg_id = obj.get("id")  # may be None
        error_obj = obj.get("error")
        result_obj = obj.get("result")

        error: ResponseError | None = None
        if error_obj is not None:
            if not isinstance(error_obj, dict):
                raise JsonRpcError(
                    INVALID_REQUEST,
                    'Invalid Request: "error" must be a JSON object',
                )
            code = error_obj.get("code")
            err_msg = error_obj.get("message", "")
            if not isinstance(code, int):
                raise JsonRpcError(
                    INVALID_REQUEST,
                    'Invalid Request: error "code" must be an integer',
                )
            error = ResponseError(
                code=code,
                message=err_msg,
                data=error_obj.get("data"),
            )

        return Response(id=msg_id, result=result_obj, error=error)

    # Requests and Notifications both have "method". The difference is "id".
    method = obj.get("method")
    if not isinstance(method, str):
        raise JsonRpcError(
            INVALID_REQUEST,
            'Invalid Request: "method" must be a string',
        )

    params = obj.get("params")  # optional; None if absent

    if "id" in obj:
        # A Request has an "id". The id must be a string or integer (not null).
        msg_id = obj["id"]
        if not isinstance(msg_id, (str, int)):
            raise JsonRpcError(
                INVALID_REQUEST,
                'Invalid Request: "id" must be a string or integer',
            )
        return Request(id=msg_id, method=method, params=params)

    # No "id" → Notification.
    return Notification(method=method, params=params)


def message_to_dict(msg: Message) -> dict[str, Any]:
    """Serialize a Message to a plain Python dict suitable for ``json.dumps``.

    This is the inverse of ``parse_message``. It produces the wire-format
    JSON object for each message type.

    The ``"jsonrpc": "2.0"`` marker is always included.

    Args:
        msg: A ``Request``, ``Response``, or ``Notification`` instance.

    Returns:
        A dict ready for ``json.dumps``.

    Raises:
        TypeError: If ``msg`` is not a recognized Message type.

    Examples::

        message_to_dict(Request(id=1, method="foo"))
        # → {"jsonrpc": "2.0", "id": 1, "method": "foo"}

        message_to_dict(Response(id=1, result=42))
        # → {"jsonrpc": "2.0", "id": 1, "result": 42}

        message_to_dict(Notification(method="bar", params={"x": 1}))
        # → {"jsonrpc": "2.0", "method": "bar", "params": {"x": 1}}
    """
    if isinstance(msg, Request):
        d: dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": msg.id,
            "method": msg.method,
        }
        if msg.params is not None:
            d["params"] = msg.params
        return d

    if isinstance(msg, Response):
        d = {"jsonrpc": "2.0", "id": msg.id}
        if msg.error is not None:
            # Error response: include the error object, omit result.
            err: dict[str, Any] = {
                "code": msg.error.code,
                "message": msg.error.message,
            }
            if msg.error.data is not None:
                err["data"] = msg.error.data
            d["error"] = err
        else:
            # Success response: include result (may be None/null).
            d["result"] = msg.result
        return d

    if isinstance(msg, Notification):
        d = {"jsonrpc": "2.0", "method": msg.method}
        if msg.params is not None:
            d["params"] = msg.params
        return d

    raise TypeError(f"Unknown message type: {type(msg)!r}")
