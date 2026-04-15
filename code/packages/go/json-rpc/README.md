# json-rpc (Go)

JSON-RPC 2.0 over stdin/stdout with Content-Length framing — the wire
protocol beneath the Language Server Protocol (LSP).

## What is this?

Package `jsonrpc` implements the JSON-RPC 2.0 specification. It is the
transport layer that every LSP server in coding-adventures builds on top of.
The LSP server for Brainfuck (and all future language servers) registers
method handlers here and lets this library handle framing, dispatch, and error
responses.

## Installation

```
module github.com/your-module

require github.com/coding-adventures/json-rpc v0.0.0
```

## Quick start

```go
package main

import (
    "os"
    jsonrpc "github.com/coding-adventures/json-rpc"
)

func main() {
    server := jsonrpc.NewServer(os.Stdin, os.Stdout)

    server.OnRequest("initialize", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
        return map[string]interface{}{"capabilities": map[string]interface{}{}}, nil
    })

    server.OnRequest("shutdown", func(id, params interface{}) (interface{}, *jsonrpc.ResponseError) {
        return nil, nil
    })

    server.OnNotification("textDocument/didOpen", func(params interface{}) {
        // handle document open
    })

    server.Serve() // blocks until stdin closes
}
```

## Message types

All messages carry `"jsonrpc": "2.0"` on the wire.

| Type           | Has `id`? | Has `method`? | Has `result`/`error`? | Direction     |
|----------------|-----------|---------------|-----------------------|---------------|
| `*Request`     | Yes       | Yes           | No                    | Client→Server |
| `*Response`    | Yes       | No            | Yes                   | Server→Client |
| `*Notification`| No        | Yes           | No                    | Client→Server |

## Wire format

```
Content-Length: <n>\r\n
\r\n
<UTF-8 JSON payload, exactly n bytes>
```

`Content-Length` is a byte count (not character count). For Unicode-heavy
payloads (emoji, CJK) this distinction matters.

## Error codes

```go
jsonrpc.ParseError     // -32700: payload is not valid JSON
jsonrpc.InvalidRequest // -32600: valid JSON but not a Request object
jsonrpc.MethodNotFound // -32601: method has no registered handler
jsonrpc.InvalidParams  // -32602: wrong parameter shape for a method
jsonrpc.InternalError  // -32603: unhandled error inside a handler
```

## API reference

### `NewReader(r io.Reader) *MessageReader`

```go
reader := jsonrpc.NewReader(os.Stdin)
msg, err := reader.ReadMessage()  // returns (Message, error); error is io.EOF on clean close
raw, err := reader.ReadRaw()      // returns raw JSON string
```

### `NewWriter(w io.Writer) *MessageWriter`

```go
writer := jsonrpc.NewWriter(os.Stdout)
err := writer.WriteMessage(response)    // typed message
err = writer.WriteRaw(`{"jsonrpc":"2.0",...}`)  // raw JSON string
```

### `NewServer(in io.Reader, out io.Writer) *Server`

```go
server := jsonrpc.NewServer(os.Stdin, os.Stdout)
server.OnRequest("method", handler)           // handler: func(id, params interface{}) (interface{}, *ResponseError)
server.OnNotification("event", handler)       // handler: func(params interface{})
server.Serve()  // blocks until EOF
```

## How it fits in the stack

```
code editor (client)
    │
    │  stdin/stdout (Content-Length framed JSON-RPC)
    │
    ▼
json-rpc (this package)
    │
    │  typed Message objects
    │
    ▼
lsp-server (future package)
    │
    │  AST, diagnostics, hover info
    │
    ▼
brainfuck (lexer + parser + interpreter)
```

## Dependencies

None. Standard library only (`encoding/json`, `bufio`, `io`, `fmt`).
