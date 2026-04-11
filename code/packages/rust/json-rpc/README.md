# coding-adventures-json-rpc

JSON-RPC 2.0 over stdin/stdout with Content-Length framing. The transport layer for all Language Server Protocol (LSP) servers in coding-adventures.

## Features

- Four message types: `Request`, `Response`, `Notification`, `ResponseError`
- Content-Length framing compatible with VS Code, Neovim, Emacs, and any LSP client
- `MessageReader<R: BufRead>` and `MessageWriter<W: Write>` — generic over any I/O type
- `Server` with `on_request` / `on_notification` method registration
- `serde` + `serde_json` for JSON — zero other dependencies

## Usage

```rust
use coding_adventures_json_rpc::Server;
use std::io::{BufReader, BufWriter};

let mut server = Server::new(
    BufReader::new(std::io::stdin()),
    BufWriter::new(std::io::stdout()),
);

server.on_request("initialize", |_id, _params| {
    Ok(serde_json::json!({"capabilities": {"hoverProvider": true}}))
});
server.on_notification("textDocument/didOpen", |params| {
    eprintln!("opened: {:?}", params);
});

server.serve(); // blocks until stdin closes
```

## Module Structure

| Module | Role |
|--------|------|
| `errors` | Standard error codes + `ResponseError` |
| `message` | `Message` enum, structs, parse/serialize |
| `reader` | `MessageReader<R>` |
| `writer` | `MessageWriter<W>` |
| `server` | `Server` dispatch loop |
