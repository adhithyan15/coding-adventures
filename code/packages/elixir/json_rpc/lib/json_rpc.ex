defmodule CodingAdventures.JsonRpc do
  @moduledoc """
  JSON-RPC 2.0 over stdin/stdout with Content-Length framing.

  ## What is JSON-RPC 2.0?

  JSON-RPC 2.0 is a stateless, lightweight remote procedure call protocol. It
  is the wire protocol underneath the Language Server Protocol (LSP) and the
  Debug Adapter Protocol (DAP). Any editor tooling — diagnostics, hover,
  go-to-definition — goes over this wire format.

  The protocol is simple:
  - A *client* sends `Request` messages (expecting a response) and
    `Notification` messages (fire-and-forget).
  - A *server* receives messages, calls registered handlers, and sends back
    `Response` messages.
  - Both sides identify messages by an `id` field. Notifications have no `id`.

  ## Content-Length Framing

  Messages travel over a raw byte stream (stdin/stdout). To tell where one
  message ends and the next begins, each message is preceded by an HTTP-style
  header:

  ```
  Content-Length: 47\\r\\n
  \\r\\n
  {"jsonrpc":"2.0","id":1,"method":"initialize"}
  ```

  The `MessageReader` reads the header, learns the exact payload size, and
  reads exactly that many bytes — no over-reading, no JSON-depth heuristics.

  ## Building an LSP Server

  ```elixir
  alias CodingAdventures.JsonRpc

  server =
    JsonRpc.Server.new(:stdio, :stdio)
    |> JsonRpc.Server.on_request("initialize", fn _id, _params ->
      %{"capabilities" => %{"hoverProvider" => true}}
    end)
    |> JsonRpc.Server.on_notification("textDocument/didOpen", fn params ->
      uri = get_in(params, ["textDocument", "uri"])
      IO.puts("opened: " <> uri)
    end)

  JsonRpc.Server.serve(server)   # blocks until stdin closes
  ```

  ## Module Map

  | Module       | Role                                          |
  |--------------|-----------------------------------------------|
  | `JsonRpc`    | Top-level convenience re-exports (this file)  |
  | `Message`    | Structs + parse_message/message_to_map        |
  | `Reader`     | MessageReader — reads framed messages         |
  | `Writer`     | MessageWriter — writes framed messages        |
  | `Server`     | Dispatch loop                                 |
  | `Errors`     | Standard error code constants + constructors  |
  | `JsonCodec`  | Internal JSON encoder/decoder (stdlib only)   |
  """

  # Re-export the most-used types for ergonomic aliasing.
  alias CodingAdventures.JsonRpc.{Message, Reader, Writer, Server, Errors}

  @doc """
  Parse a JSON binary into a typed message struct.

  Delegates to `Message.parse_message/1`.
  """
  defdelegate parse_message(json), to: Message

  @doc """
  Convert a typed message struct to a plain map for JSON encoding.

  Delegates to `Message.message_to_map/1`.
  """
  defdelegate message_to_map(message), to: Message

  @doc """
  Create a new MessageReader from an I/O device.

  Delegates to `Reader.new/1`.
  """
  defdelegate new_reader(device), to: Reader, as: :new

  @doc """
  Create a new MessageWriter from an I/O device.

  Delegates to `Writer.new/1`.
  """
  defdelegate new_writer(device), to: Writer, as: :new

  @doc """
  Create a new Server from two I/O devices.

  Delegates to `Server.new/2`.
  """
  defdelegate new_server(in_device, out_device), to: Server, as: :new

  # Convenience error constructors — delegate to the Errors module.
  # We define them explicitly (rather than defdelegate with defaults) because
  # defdelegate does not support default argument syntax in all Elixir versions.

  @doc "Build a parse-error map. Delegates to `Errors.make_parse_error/1`."
  def make_parse_error(data \\ nil), do: Errors.make_parse_error(data)

  @doc "Build an invalid-request error map. Delegates to `Errors.make_invalid_request/1`."
  def make_invalid_request(data \\ nil), do: Errors.make_invalid_request(data)

  @doc "Build a method-not-found error map. Delegates to `Errors.make_method_not_found/1`."
  def make_method_not_found(data \\ nil), do: Errors.make_method_not_found(data)

  @doc "Build an invalid-params error map. Delegates to `Errors.make_invalid_params/1`."
  def make_invalid_params(data \\ nil), do: Errors.make_invalid_params(data)

  @doc "Build an internal-error map. Delegates to `Errors.make_internal_error/1`."
  def make_internal_error(data \\ nil), do: Errors.make_internal_error(data)
end
