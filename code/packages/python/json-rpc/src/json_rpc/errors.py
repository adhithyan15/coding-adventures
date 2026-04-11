"""JSON-RPC 2.0 Standard Error Codes.

The JSON-RPC 2.0 specification reserves a range of error codes for
well-known failure modes. Think of these like HTTP status codes — a
small vocabulary of integers that a receiver can switch on without
needing to parse the human-readable ``message`` string.

Standard Error Code Table
--------------------------

    +---------------------------+----------+---------------------------------------+
    | Name                      | Code     | When to use                           |
    +---------------------------+----------+---------------------------------------+
    | PARSE_ERROR               | -32700   | Payload is not valid JSON             |
    | INVALID_REQUEST           | -32600   | Valid JSON but not a Request object   |
    | METHOD_NOT_FOUND          | -32601   | Method not registered on this server  |
    | INVALID_PARAMS            | -32602   | Handler received wrong parameter shape|
    | INTERNAL_ERROR            | -32603   | Unexpected server-side error          |
    +---------------------------+----------+---------------------------------------+

Server-defined errors live in [-32099, -32000]. For example, a Brainfuck
LSP server might define ``-32000`` as "parse failed — unmatched bracket".

LSP reserves [-32899, -32800] for protocol-level errors; the JSON-RPC
layer should never emit codes in that range.

Why negative integers?
-----------------------

The JSON-RPC spec uses negative codes to avoid collision with any
application-defined positive error codes. It is a convention, not a
technical requirement.
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Standard error codes (JSON-RPC 2.0 spec, Section 5.1)
# ---------------------------------------------------------------------------

PARSE_ERROR: int = -32700
"""The incoming message body could not be parsed as JSON.

This happens before the message type is even determined — the bytes on
the wire are not valid UTF-8 JSON. Example: ``Content-Length: 5\r\n\r\nhello``.
"""

INVALID_REQUEST: int = -32600
"""The JSON was valid, but the object does not satisfy the Request schema.

Examples: missing ``jsonrpc`` field, wrong ``jsonrpc`` value, ``id`` is
``null`` in a Request, ``method`` is not a string.
"""

METHOD_NOT_FOUND: int = -32601
"""The requested method has no registered handler.

The server understood the request, but it does not know how to handle
the ``method`` name. Equivalent to HTTP 404 for a procedure call.
"""

INVALID_PARAMS: int = -32602
"""The method exists but the provided ``params`` have the wrong shape.

Use this when a handler validates its own parameter structure and finds
missing required fields or wrong types. Equivalent to HTTP 422.
"""

INTERNAL_ERROR: int = -32603
"""An unexpected error occurred inside the server.

Use this as a catch-all when a handler raises an unhandled exception.
Equivalent to HTTP 500.
"""
