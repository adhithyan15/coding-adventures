# @coding-adventures/json-rpc

JSON-RPC 2.0 over stdin/stdout with Content-Length framing.

This package implements the transport layer beneath the Language Server Protocol
(LSP). Any LSP server in this repository delegates all message framing and
dispatch to this package, keeping the LSP layer thin.

## What is JSON-RPC 2.0?

JSON-RPC is a stateless, lightweight remote procedure call protocol. A client
sends a **Request** (a JSON object with an `id` and a `method` name), and the
server replies with a **Response** (matching `id`, plus `result` or `error`).

For one-way events that need no reply, the client sends a **Notification** — a
JSON object with a `method` but no `id`. The server must not respond.

## Wire Format

Every message is preceded by an HTTP-inspired header block:

```
Content-Length: 97\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
```

The `Content-Length` is the **byte** length of the UTF-8-encoded JSON payload.

## Architecture

```
stdin  →  MessageReader  →  Server (dispatch)  →  MessageWriter  →  stdout
                                   ↓
                           onRequest / onNotification handlers
```

## Quick Start

```typescript
import { Server } from "@coding-adventures/json-rpc";

new Server(process.stdin, process.stdout)
  .onRequest("initialize", (_id, _params) => ({
    capabilities: { hoverProvider: true },
  }))
  .onNotification("textDocument/didOpen", (params) => {
    const p = params as { textDocument: { uri: string } };
    console.error("opened:", p.textDocument.uri);
  })
  .serve();
```

## API

### `MessageReader`

Reads one framed message from a `Readable` stream.

```typescript
const reader = new MessageReader(process.stdin);
let msg: Message | null;
while ((msg = await reader.readMessage()) !== null) {
  console.log(msg);
}
```

- `readMessage()` — returns `Promise<Message | null>` (null = EOF)
- `readRaw()` — returns the raw JSON string without parsing

### `MessageWriter`

Writes one message to a `Writable` stream with Content-Length framing.

```typescript
const writer = new MessageWriter(process.stdout);
writer.writeMessage({ type: "response", id: 1, result: { ok: true } });
writer.writeRaw('{"jsonrpc":"2.0","id":2,"result":null}');
```

### `Server`

Combines reader + writer with a method dispatch table.

```typescript
const server = new Server(inStream, outStream);

server
  .onRequest("method/name", (id, params) => {
    // return result value, or a ResponseError
    return { data: 42 };
  })
  .onNotification("event/name", (params) => {
    // no return value; no response is sent
  });

await server.serve();
```

Dispatch rules:

| Situation | Action |
|-----------|--------|
| Request, handler found | Call handler; send `result` or `error` |
| Request, no handler | Send `-32601 Method not found` |
| Request, handler throws | Send `-32603 Internal error` |
| Notification, handler found | Call handler; send nothing |
| Notification, no handler | Silently ignore |

### Error Codes

```typescript
import { ErrorCodes } from "@coding-adventures/json-rpc";

ErrorCodes.ParseError     // -32700
ErrorCodes.InvalidRequest // -32600
ErrorCodes.MethodNotFound // -32601
ErrorCodes.InvalidParams  // -32602
ErrorCodes.InternalError  // -32603
```

### Message Types

```typescript
type Message = Request | Notification | Response;

// Discriminated by the `type` field:
{ type: "request",      id, method, params? }
{ type: "notification", method, params? }
{ type: "response",     id, result? | error? }
```

## Relationship to LSP

The Language Server Protocol sits on top of JSON-RPC. This package handles all
framing and dispatch; the LSP layer only registers handlers for LSP-specific
methods (`initialize`, `textDocument/hover`, etc.).

## No Dependencies

This package depends only on Node.js built-in modules (`node:stream`). It has
no runtime dependencies on any other coding-adventures package.
