defmodule Rpc.Codec do
  @moduledoc """
  Behaviour for translating between `Rpc.Message.t()` structs and raw bytes.

  ## What is a Codec?

  Think of a codec as a translator. Given an RPC message (a structured Elixir
  value), the codec turns it into bytes that can travel over a wire. Given
  bytes that arrived from the wire, the codec turns them back into a structured
  RPC message.

  The codec is stateless — it receives exactly the payload bytes (no framing
  envelope, no Content-Length header) and returns exactly the payload bytes. It
  never looks at framing; that is the `Rpc.Framer`'s job.

  ## Separation of Concerns

  ```
  RpcMessage ──[encode]──► bytes ──[write_frame]──► wire
  wire ──[read_frame]──► bytes ──[decode]──► RpcMessage
  ```

  The codec sits between the abstract message struct and the raw byte chunk. It
  does not know how many messages are in the stream, or where one ends and the
  next begins — the framer handles that.

  ## Implementing a Codec

  To build a `JsonCodec`, implement this behaviour:

  ```elixir
  defmodule MyJsonCodec do
    @behaviour Rpc.Codec

    @impl Rpc.Codec
    def encode(%Rpc.Message.Request{} = req) do
      # Serialize to JSON bytes.
      {:ok, Jason.encode!(%{"method" => req.method, ...})}
    end

    @impl Rpc.Codec
    def decode(bytes) do
      case Jason.decode(bytes) do
        {:ok, map} -> discriminate(map)
        {:error, _} -> {:error, Rpc.Errors.make_parse_error()}
      end
    end

    defp discriminate(%{"method" => _} = map), do: ...
  end
  ```

  ## Callback Contracts

  ### `encode/1`

  Receives a `Rpc.Message.t()` struct and returns `{:ok, binary()}` on
  success, or `{:error, term()}` if serialization fails (e.g., the result
  contains an un-serializable value).

  ### `decode/1`

  Receives a binary (the raw payload bytes from the framer) and returns:
  - `{:ok, %Rpc.Message.t{}}` — successfully decoded an RPC message.
  - `{:error, %Rpc.ErrorResponse{}}` — the bytes were malformed (parse error
    at code `-32700`) or valid but not a recognizable RPC message (invalid
    request at code `-32600`). The error response includes a `nil` id because
    the request id could not always be determined.
  """

  alias Rpc.Message

  # ---------------------------------------------------------------------------
  # Callbacks
  # ---------------------------------------------------------------------------

  @doc """
  Encode an `Rpc.Message.t()` struct into raw bytes.

  The returned binary is a complete, self-contained payload — no framing
  envelope is included. The caller (typically the server or client loop) passes
  these bytes to `Rpc.Framer.write_frame/2`.

  Returns `{:ok, binary()}` on success, `{:error, term()}` on failure.
  """
  @callback encode(msg :: Message.t()) :: {:ok, binary()} | {:error, term()}

  @doc """
  Decode raw bytes into an `Rpc.Message.t()` struct.

  The `data` binary is the exact payload produced by `Rpc.Framer.read_frame/1`
  — no framing envelope, no headers. The codec must determine which message
  type the bytes represent and return the appropriate struct.

  Returns:
  - `{:ok, Rpc.Message.t()}` — successfully decoded.
  - `{:error, Rpc.Message.ErrorResponse.t()}` — decoding failed. The error
    response uses a `nil` id when the request id could not be extracted.
  """
  @callback decode(data :: binary()) ::
              {:ok, Message.t()} | {:error, Message.ErrorResponse.t()}
end
