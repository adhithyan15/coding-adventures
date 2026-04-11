defmodule CodingAdventures.JsonRpc.Reader do
  @moduledoc """
  MessageReader — reads Content-Length-framed JSON-RPC messages from a device.

  ## The Framing Protocol

  LSP (and JSON-RPC over stdin/stdout) uses HTTP-inspired header framing to
  delimit messages on a byte stream. Each message looks like:

  ```
  Content-Length: 47\\r\\n
  \\r\\n
  {"jsonrpc":"2.0","id":1,"method":"initialize"}
  ```

  The header block ends at the blank line (`\\r\\n`). The payload is exactly the
  number of bytes specified by `Content-Length`. We read that many bytes and
  stop — no guessing, no searching for a delimiter.

  ### Why Content-Length framing?

  JSON has no self-delimiting structure at the byte-stream level. You cannot
  tell where one JSON object ends without parsing the entire thing — and
  partial reads can hang. Content-Length solves this: the receiver reads the
  header, learns the exact byte count, and reads exactly that many bytes in a
  single call. The next header starts immediately after.

  ## Usage

      reader = Reader.new(:stdio)

      case Reader.read_message(reader) do
        {:ok, nil} ->
          # EOF — client closed the connection
          :done
        {:ok, message} ->
          # message is %Request{}, %Response{}, or %Notification{}
          handle(message)
        {:error, error_map} ->
          # Parse or framing error — the error_map has :code, :message, :data
          log_error(error_map)
      end

  ## In-Memory Testing

  For tests, pass a `StringIO` pid as the device. The reader works with any
  Erlang I/O device — anything that responds to `:file.read/2`.

      {:ok, pid} = StringIO.open(framed_bytes)
      reader = Reader.new(pid)
      {:ok, message} = Reader.read_message(reader)
  """

  alias CodingAdventures.JsonRpc.Message

  # The reader struct wraps an I/O device handle. We keep it in a struct so
  # the API is extensible (e.g., adding buffering, tracing) without breaking
  # callers.
  defstruct [:device]

  @type t :: %__MODULE__{device: any()}

  # ---------------------------------------------------------------------------
  # new/1 — create a reader from an I/O device
  # ---------------------------------------------------------------------------

  @doc """
  Create a new MessageReader from an I/O device.

  The device can be `:stdio` for the real standard input, or a `StringIO` pid
  for testing.

  ## Example

      reader = Reader.new(:stdio)
      reader = Reader.new(string_io_pid)
  """
  @spec new(any()) :: t()
  def new(device) do
    %__MODULE__{device: device}
  end

  # ---------------------------------------------------------------------------
  # read_message/1 — read and parse the next message
  # ---------------------------------------------------------------------------

  @doc """
  Read the next message from the device.

  Returns:
  - `{:ok, nil}` — EOF (the client closed the stream)
  - `{:ok, message}` — a `%Request{}`, `%Response{}`, or `%Notification{}`
  - `{:error, error_map}` — framing or parse error

  ## Example

      case Reader.read_message(reader) do
        {:ok, nil} -> :eof
        {:ok, %Message.Request{} = req} -> dispatch(req)
        {:error, e} -> IO.puts("error: " <> e.message)
      end
  """
  @spec read_message(t()) :: {:ok, Message.message() | nil} | {:error, map()}
  def read_message(%__MODULE__{device: device}) do
    case read_raw(device) do
      {:ok, nil} ->
        {:ok, nil}

      {:ok, json} ->
        Message.parse_message(json)

      {:error, _} = err ->
        err
    end
  end

  # ---------------------------------------------------------------------------
  # read_raw/1 — read the next raw JSON payload without parsing
  # ---------------------------------------------------------------------------

  @doc """
  Read the next raw JSON payload as a binary string, without parsing it into
  a message struct. Useful for testing or when the caller wants full control
  over parsing.

  Returns `{:ok, nil}` on EOF, `{:ok, json_binary}` on success, or
  `{:error, error_map}` on framing error.
  """
  @spec read_raw(any()) :: {:ok, binary() | nil} | {:error, map()}
  def read_raw(device) do
    # Step 1: Read headers one line at a time until we see the blank line.
    # Each header is terminated by \r\n. The blank line is just \r\n.
    case read_headers(device, nil) do
      {:ok, nil} ->
        # EOF before any header — clean end of stream.
        {:ok, nil}

      {:ok, content_length} ->
        # Step 2: Read exactly content_length bytes of payload.
        read_payload(device, content_length)

      {:error, _} = err ->
        err
    end
  end

  # ---------------------------------------------------------------------------
  # Private: header reading
  # ---------------------------------------------------------------------------
  #
  # We read header lines until we see the blank line ("\r\n" on its own).
  # Each call to :file.read reads one line. We extract the Content-Length
  # value from the "Content-Length: N" header and ignore all others.
  #
  # The Erlang I/O system returns lines INCLUDING the \n terminator. We strip
  # \r\n or \n at the end of each line.

  defp read_headers(device, content_length) do
    # :file.read_line returns {:ok, line} | :eof | {:error, reason}
    case :file.read_line(device) do
      :eof ->
        # EOF at the start of a new message — clean shutdown.
        {:ok, nil}

      {:error, reason} ->
        {:error,
         %{code: -32_700, message: "Parse error", data: "I/O error: #{inspect(reason)}"}}

      {:ok, line} ->
        # Strip the trailing \r\n or \n.
        stripped = line |> to_string() |> String.trim_trailing("\r\n") |> String.trim_trailing("\n")

        cond do
          stripped == "" ->
            # Blank line — header block complete.
            if content_length == nil do
              {:error,
               %{
                 code: -32_700,
                 message: "Parse error",
                 data: "missing Content-Length header"
               }}
            else
              {:ok, content_length}
            end

          String.starts_with?(stripped, "Content-Length:") ->
            len_str = stripped |> String.replace_prefix("Content-Length:", "") |> String.trim()

            case Integer.parse(len_str) do
              {n, ""} ->
                read_headers(device, n)

              _ ->
                {:error,
                 %{
                   code: -32_700,
                   message: "Parse error",
                   data: "invalid Content-Length value: #{len_str}"
                 }}
            end

          true ->
            # Other header (e.g. Content-Type) — ignore and continue.
            read_headers(device, content_length)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: payload reading
  # ---------------------------------------------------------------------------
  #
  # After the blank line we read exactly `n` bytes. :file.read(device, n)
  # returns {:ok, data} where data may be a binary or a list of bytes (on some
  # OTP versions). We normalize to binary.

  defp read_payload(device, n) do
    case :file.read(device, n) do
      :eof ->
        {:error,
         %{
           code: -32_700,
           message: "Parse error",
           data: "EOF in payload (expected #{n} bytes)"
         }}

      {:error, reason} ->
        {:error,
         %{code: -32_700, message: "Parse error", data: "I/O error: #{inspect(reason)}"}}

      {:ok, data} ->
        payload = IO.iodata_to_binary(data)

        if byte_size(payload) < n do
          {:error,
           %{
             code: -32_700,
             message: "Parse error",
             data: "short read: expected #{n} bytes, got #{byte_size(payload)}"
           }}
        else
          {:ok, payload}
        end
    end
  end
end
