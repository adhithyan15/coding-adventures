# json-rpc (Swift)

A JSON-RPC 2.0 implementation for Swift, built on top of the generic `Rpc` package.

## What is JSON-RPC?

JSON-RPC 2.0 is a stateless, lightweight remote procedure call protocol using JSON as the data format. It is the wire protocol beneath the Language Server Protocol (LSP).

Every message carries `"jsonrpc": "2.0"` and is framed with a `Content-Length` HTTP-style header:

```
Content-Length: 42\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"ping"}
```

## Architecture

```
stdin --> MessageReader --> Server.serve() dispatch loop
                                |
                          +-----+----------+
                          |                |
                       Request?        Notification?
                          |                |
                     find handler     find handler
                     call(id, params) call(params)
                     write Response   (no response!)
                          |
                          v
                     MessageWriter --> stdout
```

## Usage

```swift
import JsonRpc

let server = Server(input: FileHandle.standardInput, output: FileHandle.standardOutput)

server.onRequest("initialize") { id, params in
    return ["capabilities": [:]]
}

server.onNotification("textDocument/didOpen") { params in
    // handle document open
}

server.serve()
```

## Message Types

| Shape        | id  | method | result | error |
|-------------|-----|--------|--------|-------|
| Request     | yes | yes    | --     | --    |
| Notification| --  | yes    | --     | --    |
| Response OK | yes | --     | yes    | --    |
| Response Err| yes | --     | --     | yes   |

## Dependencies

- `Rpc` package (via relative path `../rpc`)
