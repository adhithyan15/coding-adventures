# rpc — Codec-Agnostic RPC Primitive (Elixir)

The `rpc` package is the abstract Remote Procedure Call layer that
`json_rpc` and future codec-specific packages (`msgpack_rpc`,
`protobuf_rpc`, etc.) build on top of.

It captures the *semantics* of RPC — what a message looks like, how
requests and responses are correlated by id, how methods are dispatched to
handlers, how errors are reported — without caring about how the bytes
look on the wire.

---

## Architecture

```
Application (handlers, business logic)
     │
     ▼
┌──────────────────────────────────────────────────────────┐
│  rpc (this package)                                      │
│  Rpc.Server / Rpc.Client                                 │
│  method dispatch, id correlation, error handling         │
│  handler registry, exception recovery                    │
├──────────────────────────────────────────────────────────┤
│  Rpc.Codec behaviour                                     │
│  RpcMessage ↔ bytes       ← JSON, Protobuf, MessagePack  │
├──────────────────────────────────────────────────────────┤
│  Rpc.Framer behaviour                                    │
│  byte stream ↔ frames     ← Content-Length, newline,    │
│                              length-prefix, WebSocket    │
├──────────────────────────────────────────────────────────┤
│  Transport                                               │
│  raw byte stream          ← stdio, TCP, Unix socket      │
└──────────────────────────────────────────────────────────┘
```

Each layer knows only about the layer immediately below it. The `rpc`
layer never touches byte serialization or framing. The codec never knows
about method dispatch. The framer never knows about procedure names.

---

## Module Map

| Module              | Role                                              |
|---------------------|---------------------------------------------------|
| `Rpc`               | Top-level convenience delegates                   |
| `Rpc.Message`       | Message type structs (Request, Response, …)       |
| `Rpc.Codec`         | Behaviour: encode/decode bytes ↔ messages         |
| `Rpc.Framer`        | Behaviour: split byte stream ↔ frames             |
| `Rpc.Server`        | Blocking dispatch loop                            |
| `Rpc.Client`        | Blocking request/notification sender              |
| `Rpc.Errors`        | Standard error code constants + constructors      |

---

## Quick Start

### 1. Implement `Rpc.Codec` for your wire format

```elixir
defmodule MyJsonCodec do
  @behaviour Rpc.Codec

  @impl Rpc.Codec
  def encode(msg) do
    # Serialize msg to JSON bytes
    {:ok, Jason.encode!(to_map(msg))}
  end

  @impl Rpc.Codec
  def decode(bytes) do
    # Deserialize JSON bytes to an Rpc.Message struct
    case Jason.decode(bytes) do
      {:ok, map} -> discriminate(map)
      {:error, _} -> {:error, Rpc.Errors.make_parse_error()}
    end
  end
end
```

### 2. Implement `Rpc.Framer` for your framing scheme

```elixir
defmodule MyNewlineFramer do
  @behaviour Rpc.Framer

  def new(device), do: %{device: device}

  @impl Rpc.Framer
  def read_frame(%{device: dev} = state) do
    case IO.read(dev, :line) do
      :eof -> :eof
      {:error, reason} -> {:error, reason}
      line -> {:ok, String.trim_trailing(line, "\n"), state}
    end
  end

  @impl Rpc.Framer
  def write_frame(data, %{device: dev} = state) do
    IO.binwrite(dev, data <> "\n")
    {:ok, state}
  end
end
```

### 3. Build and start a server

```elixir
handlers =
  %{}
  |> Rpc.register_request("ping", fn _id, _params -> "pong" end)
  |> Rpc.register_notification("log", fn params ->
    IO.puts(params["message"])
  end)

framer_state = MyNewlineFramer.new(:stdio)
Rpc.serve(MyJsonCodec, MyNewlineFramer, framer_state, handlers)
```

### 4. Use the client

```elixir
framer_state = MyNewlineFramer.new(:stdio)
client = Rpc.Client.new(MyJsonCodec, MyNewlineFramer, framer_state)

case Rpc.Client.request(client, "ping", nil) do
  {:ok, result, _client2} -> IO.inspect(result)
  {:error, err, _client2} -> IO.puts("Error: #{err.message}")
end
```

---

## Error Codes

| Code    | Name            | When                                    |
|---------|-----------------|-----------------------------------------|
| -32700  | Parse error     | Codec could not decode the frame bytes  |
| -32600  | Invalid request | Decoded bytes are not an RPC message    |
| -32601  | Method not found| No handler registered for the method   |
| -32602  | Invalid params  | Handler rejected the params             |
| -32603  | Internal error  | Handler raised an unexpected exception  |

---

## Exception Safety

Handler exceptions are caught by the server via `try/rescue`. A panicking
handler causes an Internal Error (`-32603`) response to be sent to the
client. The server continues processing subsequent messages — one bad
handler cannot take down the entire server.

Notification handler exceptions are silently absorbed (notifications must
not generate error responses per the RPC spec).

---

## Running Tests

```sh
mix deps.get
mix test
```

No external dependencies — stdlib and OTP only.

---

## Related Packages

| Package         | Description                                      |
|-----------------|--------------------------------------------------|
| `json_rpc`      | `rpc` + JSON codec + Content-Length framer       |
| `msgpack_rpc`   | `rpc` + MessagePack codec + length-prefix framer |
| `protobuf_rpc`  | `rpc` + Protobuf codec + length-prefix framer    |
