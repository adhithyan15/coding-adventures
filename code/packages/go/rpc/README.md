# rpc

Codec-agnostic Remote Procedure Call primitive for Go.

## What is this?

`json-rpc`, `msgpack-rpc`, and `protobuf-rpc` all share the same core
semantics: named procedures, request/response correlation, fire-and-forget
notifications, standard error codes, handler dispatch, and panic recovery.
The only things that differ are *how the bytes are encoded* (the codec) and
*how the byte stream is split into messages* (the framer).

This package captures the shared semantics. Codec-specific packages are thin
layers that provide a concrete `RpcCodec` and `RpcFramer` implementation.

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (LSP server, custom tool, test client, …)     │
├─────────────────────────────────────────────────────────────┤
│  rpc  ← this package                                        │
│  RpcServer / RpcClient                                      │
│  (method dispatch, id correlation, error handling,          │
│   handler registry, panic recovery)                         │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec                     ← JSON, Protobuf, MessagePack │
│  (RpcMessage ↔ bytes)                                       │
├─────────────────────────────────────────────────────────────┤
│  RpcFramer                    ← Content-Length, length-     │
│  (byte stream ↔ chunks)         prefix, newline, WebSocket  │
├─────────────────────────────────────────────────────────────┤
│  Transport                    ← stdin/stdout, TCP, pipe     │
└─────────────────────────────────────────────────────────────┘
```

## Module

```
github.com/coding-adventures/rpc
```

No external dependencies — stdlib only.

## Key types

| Type | Description |
|------|-------------|
| `RpcMessage[V]` | Sealed sum type: Request, Response, ErrorResponse, Notification |
| `RpcRequest[V]` | A call expecting a response (`Id`, `Method`, `Params *V`) |
| `RpcResponse[V]` | Success reply (`Id`, `Result *V`) |
| `RpcErrorResponse[V]` | Error reply (`Id`, `Code`, `Message`, `Data *V`) |
| `RpcNotification[V]` | One-way event, no response (`Method`, `Params *V`) |
| `RpcCodec[V]` | Interface: `Encode(RpcMessage[V]) ([]byte, error)` / `Decode([]byte) (RpcMessage[V], error)` |
| `RpcFramer` | Interface: `ReadFrame() ([]byte, error)` / `WriteFrame([]byte) error` |
| `RpcServer[V]` | Read-dispatch-write loop with handler registry |
| `RpcClient[V]` | Blocking request/response correlation + notify |

## Server usage

```go
// Assume myCodec and myFramer are concrete implementations from a
// codec-specific package (e.g., json-rpc).

server := rpc.NewRpcServer(myCodec, myFramer)

server.
    OnRequest("add", func(id any, params *MyValue) (*MyValue, *rpc.RpcErrorResponse[MyValue]) {
        // extract a and b from params, compute sum...
        result := MyValue{Sum: a + b}
        return &result, nil
    }).
    OnNotification("log", func(params *MyValue) {
        fmt.Println("log:", params)
    })

server.Serve() // blocks until framer returns io.EOF
```

## Client usage

```go
client := rpc.NewRpcClient(myCodec, myFramer)

// Register server-push notification handler.
client.OnNotification("progress", func(params *MyValue) {
    fmt.Println("progress:", params)
})

// Blocking request.
p := MyValue{A: 1, B: 2}
result, errResp, err := client.Request("add", &p)
if err != nil {
    log.Fatal(err)
}
if errResp != nil {
    log.Printf("error %d: %s", errResp.Code, errResp.Message)
    return
}
fmt.Println("result:", *result)

// Fire-and-forget.
_ = client.Notify("log", &MyValue{Message: "hello"})
```

## Error codes

| Constant | Value | Meaning |
|----------|-------|---------|
| `ParseError` | -32700 | Framed bytes could not be decoded |
| `InvalidRequest` | -32600 | Decoded but not a valid RPC message |
| `MethodNotFound` | -32601 | No handler for the requested method |
| `InvalidParams` | -32602 | Handler rejected the params |
| `InternalError` | -32603 | Handler panicked |

## Implementing RpcCodec

```go
type MyCodec struct{}

func (c *MyCodec) Encode(msg rpc.RpcMessage[MyValue]) ([]byte, error) {
    switch m := msg.(type) {
    case *rpc.RpcRequest[MyValue]:
        // serialize m
    case *rpc.RpcResponse[MyValue]:
        // serialize m
    case *rpc.RpcErrorResponse[MyValue]:
        // serialize m
    case *rpc.RpcNotification[MyValue]:
        // serialize m
    }
}

func (c *MyCodec) Decode(data []byte) (rpc.RpcMessage[MyValue], error) {
    // deserialize data, discriminate on key presence
    // on failure: return nil, &rpc.RpcErrorResponse[MyValue]{Code: rpc.ParseError, ...}
}
```

## Implementing RpcFramer

```go
type MyFramer struct{ rw io.ReadWriter }

func (f *MyFramer) ReadFrame() ([]byte, error) {
    // read envelope, extract payload bytes
    // return nil, io.EOF on clean close
}

func (f *MyFramer) WriteFrame(data []byte) error {
    // write envelope + data to underlying stream
}
```

## Spec

See `code/specs/rpc.md` for the full codec-agnostic RPC primitive specification.
