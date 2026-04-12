# rpc

Codec-agnostic remote procedure call primitives for Swift.

This package is the semantic layer beneath JSON-RPC and language-server
implementations. It models the RPC message shapes, the codec contract, the
framing contract, and the blocking server/client dispatch loops without making
any assumptions about the wire format.

## What lives here

- `RpcId` for string and integer request ids
- `RpcRequest`, `RpcResponse`, `RpcErrorResponse`, `RpcNotification`
- `RpcMessage` as the codec boundary type
- `RpcCodec` for bytes <-> messages
- `RpcFramer` for payload frames
- `RpcServer` and `RpcClient`
- `RpcErrorCodes` for the standard protocol error table

## Why it exists

LSP servers do not want to know about framing or transport details. They want
to register handlers for method names, send requests, receive responses, and
surface protocol errors consistently. This package provides that layer so a
future `json-rpc` package and Swift LSP implementations can stay thin.

## Typical shape

```swift
import Rpc

final class MyCodec: RpcCodec {
    typealias Value = String

    func encode(_ message: RpcMessage<String>) throws -> Data {
        fatalError("Implement a wire format here")
    }

    func decode(_ bytes: Data) throws -> RpcMessage<String> {
        fatalError("Implement a wire format here")
    }
}

final class MyFramer: RpcFramer {
    func readFrame() throws -> Data? {
        fatalError("Implement stream framing here")
    }

    func writeFrame(_ bytes: Data) throws {
        fatalError("Implement stream framing here")
    }
}
```

The server registers request and notification handlers, and the client blocks
until it receives the matching response id. Notifications are dispatched while
the client is waiting for a response.
