# coding_adventures_json_rpc

JSON-RPC 2.0 over stdin/stdout with Content-Length framing. The transport layer for all Language Server Protocol (LSP) servers in coding-adventures.

## What is JSON-RPC?

JSON-RPC 2.0 is a stateless remote procedure call protocol. It is the wire format underneath the Language Server Protocol — every editor feature (hover, diagnostics, go-to-definition) travels over this protocol.

## Features

- Four message types: `Request`, `Response`, `Notification`, and `ResponseError`
- Content-Length framing compatible with LSP editors (VS Code, Neovim, Emacs, etc.)
- `MessageReader` and `MessageWriter` for stream I/O
- `Server` with chainable `on_request` / `on_notification` registration
- Zero external dependencies — stdlib only

## Usage

```elixir
alias CodingAdventures.JsonRpc.{Server, Message.Response}

server =
  Server.new(:stdio, :stdio)
  |> Server.on_request("initialize", fn _id, _params ->
    %{"capabilities" => %{"hoverProvider" => true}}
  end)
  |> Server.on_notification("textDocument/didOpen", fn params ->
    uri = get_in(params, ["textDocument", "uri"])
    IO.puts("opened: #{uri}")
  end)

Server.serve(server)   # blocks until stdin closes
```

## Installation

```elixir
# mix.exs
defp deps do
  [{:coding_adventures_json_rpc, path: "../json_rpc"}]
end
```

## Package Structure

| File | Role |
|------|------|
| `lib/json_rpc.ex` | Top-level convenience re-exports |
| `lib/json_rpc/message.ex` | Structs + parse/encode helpers |
| `lib/json_rpc/reader.ex` | MessageReader |
| `lib/json_rpc/writer.ex` | MessageWriter |
| `lib/json_rpc/server.ex` | Dispatch loop |
| `lib/json_rpc/errors.ex` | Standard error codes |
| `lib/json_rpc/json_codec.ex` | Internal JSON encoder/decoder |
