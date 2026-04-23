defmodule Rpc do
  @moduledoc """
  Codec-agnostic RPC primitive layer.

  ## What is this package?

  `Rpc` is the abstract remote procedure call layer that `json_rpc` and future
  codec-specific packages (`msgpack_rpc`, `protobuf_rpc`, etc.) build on top of.
  It captures the RPC *semantics* — how messages are structured, how requests
  and responses are correlated by id, how methods are dispatched to handlers,
  how errors are reported — without caring about *how the bytes look on the wire*.

  ## The Three-Layer Stack

  ```
  Application (handlers, business logic)
       │
       ▼
  ┌─────────────────────────────────────────────────────────────┐
  │  Rpc (this package)                                         │
  │  Rpc.Server / Rpc.Client                                    │
  │  method dispatch, id correlation, error handling,           │
  │  handler registry, exception recovery                       │
  ├─────────────────────────────────────────────────────────────┤
  │  Rpc.Codec behaviour                                        │
  │  RpcMessage ↔ bytes         ← JSON, Protobuf, MessagePack  │
  ├─────────────────────────────────────────────────────────────┤
  │  Rpc.Framer behaviour                                       │
  │  byte stream ↔ frames       ← Content-Length, newline,     │
  │                                length-prefix, WebSocket     │
  ├─────────────────────────────────────────────────────────────┤
  │  Transport                                                  │
  │  raw byte stream            ← stdio, TCP, Unix socket       │
  └─────────────────────────────────────────────────────────────┘
  ```

  ## Module Map

  | Module              | Role                                              |
  |---------------------|---------------------------------------------------|
  | `Rpc`               | Top-level re-exports (this file)                  |
  | `Rpc.Message`       | Message type definitions (Request, Response, …)   |
  | `Rpc.Codec`         | Behaviour: encode/decode bytes ↔ messages         |
  | `Rpc.Framer`        | Behaviour: split byte stream ↔ frames             |
  | `Rpc.Server`        | Blocking dispatch loop                            |
  | `Rpc.Client`        | Blocking request/notification sender              |
  | `Rpc.Errors`        | Standard error code constants + constructors      |

  ## Quick Example

      # Implement Rpc.Codec for your wire format.
      defmodule MyCodec do
        @behaviour Rpc.Codec
        # ... encode/decode implementation ...
      end

      # Implement Rpc.Framer for your framing scheme.
      defmodule MyFramer do
        @behaviour Rpc.Framer
        # ... read_frame/write_frame implementation ...
      end

      # Build the handlers map.
      handlers =
        %{}
        |> Rpc.register_request("ping", fn _id, _params -> "pong" end)
        |> Rpc.register_notification("log", fn params -> IO.inspect(params) end)

      # Start the server.
      framer_state = MyFramer.new(:stdio, :stdio)
      Rpc.serve(MyCodec, MyFramer, framer_state, handlers)
  """

  alias Rpc.Server

  # ---------------------------------------------------------------------------
  # Convenience delegates to Rpc.Server
  # ---------------------------------------------------------------------------

  @doc """
  Start the RPC server dispatch loop.

  Delegates to `Rpc.Server.serve/4`.
  """
  @spec serve(module(), module(), term(), map()) :: :ok
  def serve(codec_module, framer_module, framer_state, handlers \\ %{}) do
    Server.serve(codec_module, framer_module, framer_state, handlers)
  end

  @doc """
  Register a request handler in a handlers map.

  Delegates to `Rpc.Server.register_request/3`.
  """
  @spec register_request(map(), String.t(), (term(), term() -> term())) :: map()
  defdelegate register_request(handlers, method, fun), to: Server

  @doc """
  Register a notification handler in a handlers map.

  Delegates to `Rpc.Server.register_notification/3`.
  """
  @spec register_notification(map(), String.t(), (term() -> term())) :: map()
  defdelegate register_notification(handlers, method, fun), to: Server
end
