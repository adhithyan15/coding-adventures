"""LSP-specific error codes.

The JSON-RPC 2.0 specification reserves error codes in the range
[-32768, -32000]. The LSP specification further reserves [-32899, -32800]
for LSP protocol-level errors.

Standard JSON-RPC error codes (from the json-rpc package)::

    -32700  PARSE_ERROR
    -32600  INVALID_REQUEST
    -32601  METHOD_NOT_FOUND
    -32602  INVALID_PARAMS
    -32603  INTERNAL_ERROR

LSP-specific error codes are listed below.

Reference:
    https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes
"""

from __future__ import annotations

# The server has received a request before the initialize handshake was
# completed. The server must reject any request (other than initialize)
# before it has been initialized.
SERVER_NOT_INITIALIZED: int = -32002

# A generic error code for unknown errors.
UNKNOWN_ERROR_CODE: int = -32001

# LSP-specific codes in the range [-32899, -32800]:

# A request failed but not due to a protocol problem. For example, the
# document requested was not found.
REQUEST_FAILED: int = -32803

# The server cancelled the request.
SERVER_CANCELLED: int = -32802

# The document content was modified before the request completed.
# The client should retry.
CONTENT_MODIFIED: int = -32801

# The client cancelled the request.
REQUEST_CANCELLED: int = -32800
