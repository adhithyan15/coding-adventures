# @coding-adventures/rpc

Codec-agnostic RPC primitive — the abstract layer that `json-rpc`,
`msgpack-rpc`, and other concrete RPC packages build on top of.

## What is this?

JSON-RPC 2.0 is one concrete instantiation of a more general idea: one process
calling named procedures on another, passing parameters and receiving results or
errors. The **serialization format** (JSON) and the **framing scheme**
(Content-Length headers) are separable concerns. The core RPC semantics —
requests, responses, notifications, error codes, method dispatch, id
correlation — are the same regardless of how the bytes look on the wire.

This package captures those semantics as pluggable TypeScript interfaces and
generic classes. `@coding-adventures/json-rpc` is a thin instantiation of
this package. Future packages (`msgpack-rpc`, `protobuf-rpc`, etc.) will be
different instantiations of the same layer.

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (LSP server, custom tool, test client, …)     │
├─────────────────────────────────────────────────────────────┤
│  @coding-adventures/rpc                                     │
│  RpcServer / RpcClient                                      │
│  (method dispatch, id correlation, error handling,          │
│   handler registry, panic recovery)                         │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec                          ← JSON, Protobuf,        │
│  (RpcMessage ↔ bytes)                 MessagePack, XML, …   │
├─────────────────────────────────────────────────────────────┤
│  RpcFramer                         ← Content-Length,        │
│  (byte stream ↔ discrete chunks)      WebSocket, newline, … │
├─────────────────────────────────────────────────────────────┤
│  Transport                         ← stdin/stdout, TCP,     │
│  (raw byte stream)                    Unix socket, pipe, …   │
└─────────────────────────────────────────────────────────────┘
```

## Installation

```sh
npm install @coding-adventures/rpc
```

## Quick Start

### Server

```typescript
import { RpcServer } from "@coding-adventures/rpc";
// Bring a codec+framer from a concrete package:
import { JsonCodec, ContentLengthFramer } from "@coding-adventures/json-rpc";

const server = new RpcServer(
  new JsonCodec(),
  new ContentLengthFramer(process.stdin, process.stdout),
);

server
  .onRequest("add", (_id, params) => {
    const { a, b } = params as { a: number; b: number };
    return a + b;
  })
  .onNotification("shutdown", () => {
    process.exit(0);
  })
  .serve(); // blocks until EOF
```

### Client

```typescript
import { RpcClient } from "@coding-adventures/rpc";
import { JsonCodec, ContentLengthFramer } from "@coding-adventures/json-rpc";

const client = new RpcClient(
  new JsonCodec(),
  new ContentLengthFramer(childProcess.stdout, childProcess.stdin),
);

// Register for server-push notifications:
client.onNotification("$/progress", (params) => {
  console.log("Progress:", params);
});

// Synchronous blocking request:
const result = client.request("add", { a: 3, b: 4 });
console.log(result); // 7

// Fire-and-forget:
client.notify("$/cancelRequest", { id: 1 });
```

## API

### Message Types

All four message shapes share a `kind` discriminant:

| Shape              | `kind`           | Fields                                           |
|--------------------|------------------|--------------------------------------------------|
| `RpcRequest<V>`    | `'request'`      | `id: RpcId`, `method: string`, `params?: V`      |
| `RpcResponse<V>`   | `'response'`     | `id: RpcId`, `result: V`                         |
| `RpcErrorResponse<V>` | `'error'`     | `id: RpcId \| null`, `code: number`, `message: string`, `data?: V` |
| `RpcNotification<V>` | `'notification'` | `method: string`, `params?: V`               |

`RpcId = string | number`

### Error Codes

| Constant                      | Value   | Meaning                                      |
|-------------------------------|---------|----------------------------------------------|
| `RpcErrorCodes.ParseError`    | -32700  | Bytes could not be decoded by the codec       |
| `RpcErrorCodes.InvalidRequest`| -32600  | Decoded but not a valid RPC message shape     |
| `RpcErrorCodes.MethodNotFound`| -32601  | No handler registered for the method          |
| `RpcErrorCodes.InvalidParams` | -32602  | Handler rejected the params                   |
| `RpcErrorCodes.InternalError` | -32603  | Unexpected exception inside a handler         |

### `RpcServer<V>`

```typescript
class RpcServer<V> {
  constructor(codec: RpcCodec<V>, framer: RpcFramer)
  onRequest(method: string, handler: (id: RpcId, params: V | undefined) => V): this
  onNotification(method: string, handler: (params: V | undefined) => void): this
  serve(): void  // synchronous blocking loop
}
```

**Dispatch rules:**
- Request with registered handler → call handler; write success response
- Request with no handler → write `-32601 MethodNotFound`
- Handler throws → write `-32603 InternalError` (server never crashes)
- Notification with registered handler → call handler; write nothing
- Unknown notification → silently drop (no error per spec)
- Decode error → write error response with `null` id; continue loop

### `RpcClient<V>`

```typescript
class RpcClient<V> {
  constructor(codec: RpcCodec<V>, framer: RpcFramer)
  onNotification(method: string, handler: (params: V | undefined) => void): this
  request(method: string, params?: V): V  // blocking
  notify(method: string, params?: V): void
}
```

- `request()` generates a monotonically increasing id (starting at 1), writes the
  request frame, then blocks reading frames until a response with a matching id arrives.
- While blocked, server-push notifications are dispatched to registered handlers.
- Throws `RpcClientError` if the server returns an error or the connection closes.

### Implementing a Codec

```typescript
import { RpcCodec, RpcMessage, RpcError, RpcErrorCodes } from "@coding-adventures/rpc";

class MyCodec implements RpcCodec<MyValue> {
  encode(msg: RpcMessage<MyValue>): Uint8Array {
    // serialize msg to bytes
  }
  decode(data: Uint8Array): RpcMessage<MyValue> {
    // deserialize bytes to msg
    // throw new RpcError(RpcErrorCodes.ParseError, "...") on parse failure
    // throw new RpcError(RpcErrorCodes.InvalidRequest, "...") on schema failure
  }
}
```

### Implementing a Framer

```typescript
import { RpcFramer } from "@coding-adventures/rpc";

class MyFramer implements RpcFramer {
  readFrame(): Uint8Array | null {
    // return null on EOF, bytes on success
    // throw RpcError on framing error
  }
  writeFrame(data: Uint8Array): void {
    // write framed data to the underlying stream
  }
}
```

## How It Relates to json-rpc

```
@coding-adventures/json-rpc = @coding-adventures/rpc + JsonCodec + ContentLengthFramer
```

The `json-rpc` package provides `JsonCodec` and `ContentLengthFramer` that
implement the interfaces defined here. The `RpcServer` and `RpcClient` in this
package do all the dispatch logic; the codec and framer just handle bytes.

## Tests

```sh
npm test
npm run test:coverage
```

Coverage target: >80% lines (actual: ~95%+).

## Spec

See [`code/specs/rpc.md`](../../../specs/rpc.md) for the full design
specification, including the layering rationale, concrete instantiation
examples, and the relationship to `json-rpc` and future codec packages.
