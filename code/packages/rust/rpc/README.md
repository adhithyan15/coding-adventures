# coding-adventures-rpc

Codec-agnostic RPC primitive for Rust. This crate captures the *semantics* of
remote procedure calls — method dispatch, id correlation, error codes, panic
recovery — without coupling to any particular serialisation format or framing
scheme.

## Where It Fits

```
┌─────────────────────────────────────────────────────────────┐
│  Application  (LSP server, CLI tool, test client, …)         │
├─────────────────────────────────────────────────────────────┤
│  coding-adventures-rpc  ← YOU ARE HERE                      │
│  RpcServer / RpcClient                                       │
│  (method dispatch, id correlation, error handling,           │
│   handler registry, panic recovery)                          │
├─────────────────────────────────────────────────────────────┤
│  RpcCodec  (pluggable: JSON, MessagePack, Protobuf, …)       │
├─────────────────────────────────────────────────────────────┤
│  RpcFramer  (pluggable: Content-Length, length-prefix, …)   │
├─────────────────────────────────────────────────────────────┤
│  Transport  (stdin/stdout, TCP, Unix socket, …)             │
└─────────────────────────────────────────────────────────────┘
```

`coding-adventures-json-rpc` is one concrete instantiation:

```
json-rpc = rpc + JsonCodec + ContentLengthFramer
```

Future packages like `msgpack-rpc` and `protobuf-rpc` will be different
instantiations of the same `rpc` layer with no changes to handler code.

## Modules

| Module    | Contents                                                      |
|-----------|---------------------------------------------------------------|
| `errors`  | `RpcError` + standard error code constants (-32700 … -32603) |
| `message` | `RpcMessage<V>`, `RpcRequest`, `RpcResponse`, `RpcNotification`, `RpcId` |
| `codec`   | `RpcCodec<V>` trait                                           |
| `framer`  | `RpcFramer` trait                                             |
| `server`  | `RpcServer<R, W, V>` — blocking dispatch loop                 |
| `client`  | `RpcClient<V>` — synchronous blocking client                  |

## Usage

### Server

```rust
use coding_adventures_rpc::server::RpcServer;
use serde_json::Value;

// Bring your own codec + framer from e.g. coding-adventures-json-rpc:
let mut server = RpcServer::new(
    Box::new(my_json_codec),
    Box::new(content_length_framer),
);

server
    .on_request("ping", |_id, _params| {
        Ok(Value::String("pong".into()))
    })
    .on_notification("log", |params| {
        eprintln!("log: {:?}", params);
    });

server.serve(); // blocks until EOF
```

### Client

```rust
use coding_adventures_rpc::client::RpcClient;
use serde_json::Value;

let mut client: RpcClient<Value> = RpcClient::new(
    Box::new(my_json_codec),
    Box::new(content_length_framer),
);

// Blocking request — waits for the matching response:
let result = client.request("ping", None)?;
println!("{}", result); // "pong"

// Fire-and-forget notification:
client.notify("log", Some(Value::String("hello".into())))?;
```

### Implementing a Codec

```rust
use coding_adventures_rpc::codec::RpcCodec;
use coding_adventures_rpc::errors::RpcError;
use coding_adventures_rpc::message::{RpcMessage, RpcErrorResponse};
use serde_json::Value;

struct MyCodec;

impl RpcCodec<Value> for MyCodec {
    fn encode(&self, msg: &RpcMessage<Value>) -> Result<Vec<u8>, RpcError> {
        // Serialise the message to bytes.
        todo!()
    }
    fn decode(&self, data: &[u8]) -> Result<RpcMessage<Value>, RpcErrorResponse<Value>> {
        // Deserialise bytes to a message, or return PARSE_ERROR / INVALID_REQUEST.
        todo!()
    }
}
```

### Implementing a Framer

```rust
use coding_adventures_rpc::framer::RpcFramer;
use coding_adventures_rpc::errors::RpcError;

struct NewlineFramer { /* reader + writer */ }

impl RpcFramer for NewlineFramer {
    fn read_frame(&mut self) -> Option<Result<Vec<u8>, RpcError>> {
        // Read one '\n'-terminated line. Return None on EOF.
        todo!()
    }
    fn write_frame(&mut self, data: &[u8]) -> Result<(), RpcError> {
        // Write data + '\n'.
        todo!()
    }
}
```

## Error Codes

| Code      | Constant           | When                                         |
|-----------|--------------------|----------------------------------------------|
| `-32700`  | `PARSE_ERROR`      | Frame bytes could not be decoded              |
| `-32600`  | `INVALID_REQUEST`  | Decoded but not a valid RPC message           |
| `-32601`  | `METHOD_NOT_FOUND` | No handler registered for the method         |
| `-32602`  | `INVALID_PARAMS`   | Handler rejected the params                  |
| `-32603`  | `INTERNAL_ERROR`   | Unexpected error inside the handler (panic)  |

## Spec

See `code/specs/rpc.md` for the full language-agnostic specification this
package implements.
