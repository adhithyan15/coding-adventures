# coding-adventures-rpc

Codec-agnostic RPC primitive for Python.

This package defines the abstract RPC layer that sits beneath codec-specific
packages like `coding-adventures-json-rpc`.  The separation of concerns lets
you swap the serialisation format (JSON, MessagePack, Protobuf) and framing
scheme (Content-Length, length-prefix, newlines) without touching the server
or client logic.

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (LSP server, custom tool, test client, …)     │
├─────────────────────────────────────────────────────────────┤
│  rpc  ← this package                                        │
│  RpcServer / RpcClient                                      │
│  (method dispatch, id correlation, error handling,          │
│   handler registry, panic recovery)                         │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec                            ← JSON, Protobuf, …    │
│  (RpcMessage ↔ bytes)                                       │
├─────────────────────────────────────────────────────────────┤
│  RpcFramer                           ← Content-Length, …    │
│  (byte stream ↔ discrete byte chunks)                       │
├─────────────────────────────────────────────────────────────┤
│  Transport  (stdin/stdout, TCP, pipe, …)                    │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```bash
pip install coding-adventures-rpc
```

## Quick Start

### Server

```python
from rpc import RpcServer

server = (
    RpcServer(codec=MyCodec(), framer=MyFramer(stream))
    .on_request("add", lambda id, params: params["a"] + params["b"])
    .on_notification("log", lambda params: print(params["msg"]))
)
server.serve()   # blocks until EOF
```

### Client

```python
from rpc import RpcClient, RpcRemoteError

client = RpcClient(codec=MyCodec(), framer=MyFramer(stream))
client.on_notification("ping", lambda p: print("server pinged us"))

try:
    result = client.request("add", {"a": 3, "b": 4})
    print(result)   # 7
except RpcRemoteError as exc:
    print(f"Error {exc.error.code}: {exc.error.message}")

client.notify("log", {"msg": "done"})
```

## Message Types

| Type                | Direction          | Has id? | Has method? |
|---------------------|--------------------|---------|-------------|
| `RpcRequest`        | Client → Server    | Yes     | Yes         |
| `RpcResponse`       | Server → Client    | Yes     | No          |
| `RpcErrorResponse`  | Server → Client    | Maybe   | No          |
| `RpcNotification`   | Either direction   | No      | Yes         |

## Error Codes

| Code     | Constant          | Meaning                                  |
|----------|-------------------|------------------------------------------|
| `-32700` | `PARSE_ERROR`     | Framed bytes could not be decoded        |
| `-32600` | `INVALID_REQUEST` | Decoded OK but not a valid RPC message   |
| `-32601` | `METHOD_NOT_FOUND`| No handler registered for method         |
| `-32602` | `INVALID_PARAMS`  | Handler rejected params as malformed     |
| `-32603` | `INTERNAL_ERROR`  | Unexpected server-side error             |

## Implementing a Codec

```python
from rpc import RpcMessage
from rpc.errors import RpcDecodeError, PARSE_ERROR

class MyCodec:
    def encode(self, msg: RpcMessage) -> bytes:
        # Serialize msg to bytes (JSON, msgpack, protobuf, etc.)
        ...

    def decode(self, data: bytes) -> RpcMessage:
        # Parse bytes back into an RpcMessage.
        # Raise RpcDecodeError on failure.
        try:
            return self._parse(data)
        except ValueError as exc:
            raise RpcDecodeError(PARSE_ERROR, str(exc)) from exc
```

## Implementing a Framer

```python
from rpc.framer import RpcFramer

class MyFramer:
    def __init__(self, stream) -> None:
        self._stream = stream

    def read_frame(self) -> bytes | None:
        # Return next payload bytes, or None on EOF.
        ...

    def write_frame(self, data: bytes) -> None:
        # Write payload bytes with framing envelope.
        ...
```

## Relationship to Other Packages

| Package                        | Depends on  | Description                    |
|-------------------------------|-------------|--------------------------------|
| `coding-adventures-rpc`       | —           | This package (abstract RPC)    |
| `coding-adventures-json-rpc`  | rpc         | JSON + Content-Length framing  |
| Future `msgpack-rpc`           | rpc         | MessagePack + length-prefix    |
| Future `protobuf-rpc`          | rpc         | Protobuf + length-prefix       |

## Development

```bash
uv venv
uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v --cov=rpc
```
