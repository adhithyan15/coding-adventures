# JSON-RPC 2.0

## Overview

JSON-RPC 2.0 is a stateless, lightweight remote procedure call protocol that uses JSON as its data format. It is the wire protocol underneath the Language Server Protocol (LSP) and is the standard transport for any editor tooling we build — diagnostics, hover, go-to-definition, and so on.

This spec defines a generic `json-rpc` package that must be implemented in all eight languages the toolchain supports: Go, TypeScript, Python, Ruby, Elixir, Rust, Lua, and Perl. Each implementation exposes the same conceptual API so that LSP servers written in any language are built the same way.

## Why JSON-RPC Before LSP?

The LSP spec sits on top of JSON-RPC. The LSP server for Brainfuck (and every future language) delegates all message framing, dispatch, and error handling to the JSON-RPC layer. Implementing JSON-RPC first:

1. Keeps the LSP layer thin — it only knows about LSP-specific methods, not framing
2. Lets us test the transport independently of any language server logic
3. Gives us a reusable library for any future RPC-based protocol (DAP, custom tools)

## Message Types

JSON-RPC 2.0 defines four message shapes. All messages carry `"jsonrpc": "2.0"`.

### Request

A call from client to server expecting a response. The `id` ties the response back to this request.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/hover",
  "params": { "textDocument": { "uri": "file:///main.bf" }, "position": { "line": 0, "character": 3 } }
}
```

- `id`: string or integer, unique per in-flight request. Must never be `null`.
- `method`: string, the procedure name.
- `params`: optional object or array.

### Response (success)

Sent by the server in reply to a Request. The `id` matches the originating request.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "contents": { "kind": "markdown", "value": "**INC** — increment current cell" } }
}
```

- `result`: any JSON value; the procedure's return value.
- A response **must not** carry both `result` and `error`.

### Response (error)

Sent when the server cannot fulfil the request.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": "textDocument/hover is not registered"
  }
}
```

- `code`: integer (see error codes below).
- `message`: short human-readable description.
- `data`: optional, any JSON value providing additional context.

### Notification

A one-way message with no response. Used for events the client fires without waiting for an answer (e.g., `textDocument/didChange`).

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": { "textDocument": { "uri": "file:///main.bf", "text": "++[>+<-]." } }
}
```

- No `id` field. A message with `id` is always a Request; a message without `id` is always a Notification.
- The server **must not** send a response to a Notification.

## Standard Error Codes

These codes are reserved by the JSON-RPC 2.0 specification:

| Code              | Name              | Meaning                                               |
|-------------------|-------------------|-------------------------------------------------------|
| `-32700`          | Parse error       | The message body is not valid JSON                   |
| `-32600`          | Invalid Request   | The JSON was parsed but is not a valid Request object |
| `-32601`          | Method not found  | The method does not exist / is not registered        |
| `-32602`          | Invalid params    | Invalid method parameters                            |
| `-32603`          | Internal error    | Internal server error                                |
| `-32000` to `-32099` | Server errors  | Reserved for implementation-defined server errors    |

LSP reserves the range `-32899` to `-32800` for LSP-specific errors. The JSON-RPC layer should not use these.

## Transport: LSP Header Framing

The default transport is **stdin/stdout** with HTTP-inspired Content-Length framing. This is the transport LSP mandates and the one all our LSP servers will use.

Each message is preceded by a header block and a blank line:

```
Content-Length: <n>\r\n
\r\n
<UTF-8 JSON payload, exactly n bytes>
```

Rules:
- `Content-Length` is the only required header. Its value is the byte length of the UTF-8-encoded JSON payload.
- The header block is separated from the payload by `\r\n` (a blank line).
- The payload is always UTF-8. The `Content-Type` header (optional) defaults to `application/vscode-jsonrpc; charset=utf-8`.
- There is no message delimiter — the next message begins immediately after the previous payload ends.
- A `MessageReader` must read the `Content-Length`, then read exactly that many bytes. It must not read a byte more.

### Why this framing?

JSON has no self-delimiting structure at the byte stream level — you cannot tell where one message ends without parsing the entire JSON. The Content-Length header solves this without requiring full JSON parsing just to find the message boundary.

## Public API

The package exposes three building blocks — `MessageReader`, `MessageWriter`, and `Server` — plus the four message types as first-class data structures.

### Message types (data structures)

```
Request {
  id:     string | integer
  method: string
  params: any   (optional)
}

Response {
  id:     string | integer | null
  result: any               (present on success)
  error:  ResponseError     (present on failure)
}

ResponseError {
  code:    integer
  message: string
  data:    any    (optional)
}

Notification {
  method: string
  params: any    (optional)
}

# Discriminated union over the four types
Message = Request | Response | Notification
```

`id: null` in a Response is only valid when the server cannot determine the request id (e.g., the request was unparseable). Otherwise the id must match.

### MessageReader

Reads one framed message from a byte stream. Returns `nil`/`None`/`null` on EOF.

```
MessageReader
  .new(stream)                 → reader
  .read_message()              → Message | nil
  .read_raw()                  → string | nil   (raw JSON, without parsing)
```

`read_message` parses the JSON and returns a typed `Message`. `read_raw` is useful for testing or when the caller wants to control parsing.

### MessageWriter

Writes one message to a byte stream, applying Content-Length framing.

```
MessageWriter
  .new(stream)                 → writer
  .write_message(message)      → void
  .write_raw(json_string)      → void
```

### Server

The server combines a reader and writer with a method dispatch table. It drives the read-dispatch-write loop.

```
Server
  .new(in_stream, out_stream)                   → server
  .on_request(method, handler)                  → server   (chainable)
  .on_notification(method, handler)             → server   (chainable)
  .serve()                                      → void     (blocking)

Handler signatures:
  request handler:      fn(id, params) → result | ResponseError
  notification handler: fn(params)     → void
```

`serve()` reads messages in a loop until EOF or an unrecoverable error. For each message:
- If it is a Request: look up the handler by `method`. If found, call it and send the result or error as a Response. If not found, send a `-32601 Method not found` error response.
- If it is a Notification: look up the handler by `method`. If found, call it. If not found, silently ignore (per JSON-RPC spec — notifications must not generate error responses).
- If it is a Response: forward to a pending-request table (for client-side usage; servers that only respond do not need this).

### Concurrency

The `serve()` loop is single-threaded and processes one message at a time. LSP editors send requests one at a time and wait for responses before sending the next (with the exception of notifications and cancellation). This simplest model is correct for our LSP servers.

Future work: if a language server needs to handle concurrent requests (e.g., for `$/cancelRequest`), a thread-pool variant of `serve()` can be added without changing the handler API.

## Package Structure

Package name: `json-rpc` in all languages, following the repo's existing convention.

```
go/json-rpc/
  reader.go      — MessageReader
  writer.go      — MessageWriter
  message.go     — Message, Request, Response, Notification, ResponseError types
  server.go      — Server
  errors.go      — Standard error code constants
  *_test.go      — Tests

typescript/json-rpc/src/
  reader.ts
  writer.ts
  message.ts
  server.ts
  errors.ts

python/json-rpc/src/json_rpc/
  reader.py
  writer.py
  message.py
  server.py
  errors.py

ruby/json_rpc/lib/coding_adventures/json_rpc/
  reader.rb
  writer.rb
  message.rb
  server.rb
  errors.rb

elixir/json_rpc/lib/json_rpc/
  reader.ex
  writer.ex
  message.ex
  server.ex
  errors.ex

rust/json-rpc/src/
  reader.rs
  writer.rs
  message.rs
  server.rs
  errors.rs

lua/json-rpc/src/coding_adventures/json_rpc/
  reader.lua
  writer.lua
  message.lua
  server.lua
  errors.lua

perl/json-rpc/lib/CodingAdventures/JsonRpc/
  Reader.pm
  Writer.pm
  Message.pm
  Server.pm
  Errors.pm
```

## Usage Example

The following pseudocode shows how an LSP server for Brainfuck would use this library:

```
server = Server.new(stdin, stdout)
  .on_request("initialize", fn(id, params) →
    { capabilities: { hoverProvider: true, diagnosticProvider: true } }
  )
  .on_notification("textDocument/didOpen", fn(params) →
    # parse params.textDocument.text with Brainfuck.Parser
    # store AST for subsequent requests
    nil
  )
  .on_request("textDocument/hover", fn(id, params) →
    # look up token at params.position in the stored AST
    # return hover content
    { contents: { kind: "plaintext", value: "..." } }
  )
  .serve()   # blocks until stdin closes
```

The LSP layer never touches framing or dispatch — it only registers handlers.

## Dependencies

- The `json-rpc` package depends only on the language's standard library (JSON parsing, I/O). It has **no** dependency on any other coding-adventures package.
- The Brainfuck LSP server will depend on `json-rpc` and `brainfuck` (lexer + parser + interpreter).

## Test Coverage Targets

- `MessageReader`: reads a single message, reads multiple back-to-back messages, returns nil on EOF, raises `Parse error (-32700)` on malformed JSON, raises `Invalid Request (-32600)` on valid JSON that is not a message
- `MessageWriter`: writes with correct `Content-Length` header, UTF-8 payload, `\r\n` separator
- `Server`: dispatches request to handler and writes response, dispatches notification to handler without writing response, sends `-32601` for unknown method, sends error response when handler returns `ResponseError`

## Relationship to LSP

The Language Server Protocol spec (future `code/specs/lsp-server.md`) will build on top of this. The JSON-RPC layer is protocol-agnostic — it knows nothing about LSP method names, parameter shapes, or lifecycle. The LSP layer is purely a collection of handler registrations on top of a `Server` instance.
