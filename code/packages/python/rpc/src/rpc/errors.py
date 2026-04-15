"""RPC Standard Error Codes and Exceptions.

The RPC specification defines a small vocabulary of integer error codes that
both client and server understand without needing to parse the human-readable
``message`` string.  Think of them like HTTP status codes: ``404`` is
"not found" whether the response body says "Not Found", "Nincs találat",
or "リソースが見つかりません".

Standard Error Code Table
--------------------------

    +---------------------------+----------+---------------------------------------+
    | Name                      | Code     | When to use                           |
    +---------------------------+----------+---------------------------------------+
    | PARSE_ERROR               | -32700   | Framed bytes could not be decoded     |
    | INVALID_REQUEST           | -32600   | Decoded OK but not a valid RPC msg    |
    | METHOD_NOT_FOUND          | -32601   | No handler registered for method      |
    | INVALID_PARAMS            | -32602   | Handler rejected params as malformed  |
    | INTERNAL_ERROR            | -32603   | Unexpected server-side error          |
    +---------------------------+----------+---------------------------------------+

These codes are inherited from JSON-RPC 2.0 and are codec-agnostic.
The same integers apply whether you encode with JSON, MessagePack, or Protobuf.

Why negative integers?
-----------------------

The original JSON-RPC spec chose negative numbers to avoid collision with
any application-defined *positive* error codes.  It is a convention, not a
technical requirement.  Most RPC frameworks that extend this spec follow the
same convention: application errors are positive, protocol errors are negative.

Server-defined errors
----------------------

The range ``[-32099, -32000]`` is reserved for implementation-defined server
errors.  For example, a hypothetical Brainfuck language server might use
``-32000`` to mean "parse failed — unmatched bracket".

The range ``[-32899, -32800]`` is reserved by LSP for protocol-level errors.
The ``rpc`` layer must never emit codes in that range; those belong to the
application layer above.

RpcDecodeError
--------------

``RpcDecodeError`` is raised by :class:`~rpc.codec.RpcCodec` implementations
when ``decode()`` is called with bytes that cannot be decoded into a valid
:class:`~rpc.message.RpcMessage`.  The server catches it, constructs an
:class:`~rpc.message.RpcErrorResponse` with the appropriate code, and continues
the serve loop.

Example::

    class MyCodec:
        def decode(self, data: bytes) -> RpcMessage:
            try:
                return self._parse(data)
            except ValueError as exc:
                raise RpcDecodeError(PARSE_ERROR, f"parse error: {exc}") from exc
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Standard error codes (JSON-RPC 2.0 spec, Section 5.1 — codec-agnostic)
# ---------------------------------------------------------------------------

PARSE_ERROR: int = -32700
"""The framed bytes could not be decoded by the codec.

This happens before the message type is even determined — the raw bytes
arriving from the framer are not valid for the codec (e.g., not valid JSON,
not valid MessagePack).  Example in JSON: ``Content-Length: 5\\r\\n\\r\\nhello``.
"""

INVALID_REQUEST: int = -32600
"""Decoded successfully but the object does not satisfy the RPC message schema.

The bytes were valid codec payload (valid JSON / valid MessagePack / etc.) but
the resulting object is not a recognizable RPC message.  Examples: missing
``method`` field in a request, ``id`` is ``null`` in a request, ``result`` and
``error`` both present in a response.
"""

METHOD_NOT_FOUND: int = -32601
"""The requested method has no registered handler.

The server understood the request, but its dispatch table contains no entry for
the ``method`` name.  Equivalent to HTTP 404 for a procedure call.
"""

INVALID_PARAMS: int = -32602
"""The method exists but the provided ``params`` have the wrong shape.

Use this when a handler validates its own parameter structure and finds missing
required fields or wrong types.  Equivalent to HTTP 422 Unprocessable Entity.
"""

INTERNAL_ERROR: int = -32603
"""An unexpected error occurred inside the server handler.

Use this as a catch-all when a handler raises an unhandled exception or panics.
Equivalent to HTTP 500 Internal Server Error.
"""


# ---------------------------------------------------------------------------
# Exception type raised by RpcCodec.decode()
# ---------------------------------------------------------------------------


class RpcDecodeError(Exception):
    """Raised by :meth:`~rpc.codec.RpcCodec.decode` when bytes cannot be decoded.

    The ``code`` attribute holds one of the standard error code constants
    (typically :data:`PARSE_ERROR` or :data:`INVALID_REQUEST`) so that the
    server can include the exact code in the error response it sends back.

    Attributes:
        code: Integer error code (see constants above).
        message: Human-readable description of what went wrong.

    Example::

        try:
            msg = codec.decode(raw_bytes)
        except RpcDecodeError as exc:
            print(f"Decode failed: code={exc.code}, msg={exc.message}")
            # → Decode failed: code=-32700, msg=parse error: unexpected byte
    """

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code: int = code
        self.message: str = message

    def __repr__(self) -> str:
        return f"RpcDecodeError(code={self.code!r}, message={self.message!r})"
