defmodule CodingAdventures.JsonRpc.Writer do
  @moduledoc """
  MessageWriter — writes Content-Length-framed JSON-RPC messages to a device.

  ## The Framing Protocol

  Every message is prefixed with an HTTP-style header block and a blank line:

  ```
  Content-Length: 47\\r\\n
  \\r\\n
  {"jsonrpc":"2.0","id":1,"result":{"ok":true}}
  ```

  Steps:
  1. Encode the message struct to a JSON binary (UTF-8).
  2. Compute `byte_size(json)` — this is the Content-Length value. We use
     `byte_size` not `String.length` because UTF-8 characters can be 1–4 bytes.
  3. Write: `"Content-Length: N\\r\\n\\r\\n"` followed by the JSON bytes.

  ## Usage

      writer = Writer.new(:stdio)
      :ok = Writer.write_message(writer, %Response{id: 1, result: %{"ok" => true}})

  ## In-Memory Testing

      {:ok, pid} = StringIO.open("")
      writer = Writer.new(pid)
      :ok = Writer.write_message(writer, %Notification{method: "initialized"})
      {_in, out} = StringIO.contents(pid)
      # out contains the framed message bytes
  """

  alias CodingAdventures.JsonRpc.{JsonCodec, Message}

  defstruct [:device]

  @type t :: %__MODULE__{device: any()}

  # ---------------------------------------------------------------------------
  # new/1
  # ---------------------------------------------------------------------------

  @doc """
  Create a new MessageWriter from an I/O device.

      writer = Writer.new(:stdio)
  """
  @spec new(any()) :: t()
  def new(device) do
    %__MODULE__{device: device}
  end

  # ---------------------------------------------------------------------------
  # write_message/2 — encode a typed message and write it with framing
  # ---------------------------------------------------------------------------

  @doc """
  Write a typed message struct to the device with Content-Length framing.

  Converts the message to a plain map, encodes it as JSON, computes the byte
  length, and writes the header + payload.

  Returns `:ok` on success or `{:error, reason}` on failure.

  ## Example

      :ok = Writer.write_message(writer, %Response{id: 1, result: nil})
  """
  @spec write_message(t(), Message.message()) :: :ok | {:error, any()}
  def write_message(%__MODULE__{device: device}, message) do
    map = Message.message_to_map(message)

    case JsonCodec.encode(map) do
      {:error, reason} ->
        {:error, reason}

      {:ok, json} ->
        write_raw(device, json)
    end
  end

  # ---------------------------------------------------------------------------
  # write_raw/2 — write a raw JSON binary with framing
  # ---------------------------------------------------------------------------

  @doc """
  Write a raw JSON binary string with Content-Length framing. Useful when
  the caller has already encoded the message or wants to write a custom payload.

  Returns `:ok` on success or `{:error, reason}` on failure.

  ## Example

      :ok = Writer.write_raw(writer, ~s({"jsonrpc":"2.0","id":1,"result":42}))
  """
  @spec write_raw(any(), binary()) :: :ok | {:error, any()}
  def write_raw(device, json) when is_binary(json) do
    # The Content-Length value is the BYTE length, not the character count.
    # For ASCII-only JSON this is the same, but for JSON with Unicode (e.g.
    # file paths with non-ASCII characters), byte_size and String.length differ.
    n = byte_size(json)
    header = "Content-Length: #{n}\r\n\r\n"
    payload = header <> json

    case IO.binwrite(device, payload) do
      :ok -> :ok
      {:error, _} = err -> err
    end
  end
end
