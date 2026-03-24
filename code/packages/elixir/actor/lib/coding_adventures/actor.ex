defmodule CodingAdventures.Actor do
  @moduledoc """
  # Actor — The Foundation of Concurrent Computation

  ## What Is the Actor Model?

  The Actor model is a mathematical framework for concurrent computation invented by
  Carl Hewitt, Peter Bishop, and Richard Steiger in 1973. It defines computation in
  terms of **actors** — independent entities that communicate exclusively through
  **messages**. No shared memory. No locks. No mutexes. Just isolated units of
  computation passing immutable messages through one-way channels.

  **Analogy:** Think of a congressional office. Each staffer is an actor. The Staff
  Assistant receives phone calls (messages), writes call summaries (new messages),
  and sends them to the Communications Director (another actor). The Staff Assistant
  cannot reach into the Communications Director's desk and read their draft press
  release. They can only send a message asking for it.

  ## Three Primitives

  This module implements three primitives sufficient to build any concurrent system:

  1. **Message** — the atom of communication. Immutable, typed, serializable.
  2. **Channel** — a one-way, append-only pipe for messages. Persistent and replayable.
  3. **Actor** — an isolated unit of computation with a mailbox and internal state.

  Plus an **ActorSystem** that manages lifecycles, message delivery, and channels.

  ## Architecture

  ```
  User Programs / Chief of Staff (D18)
  │   create_actor(behavior)     — spawn a new actor
  │   send_message(actor, msg)   — deliver a message
  │   channel.append(message)    — publish to a channel
  │   channel.read(offset)       — consume from a channel
  ▼
  Actor Runtime ← THIS MODULE
  │   ├── Actor         — isolated computation + mailbox
  │   ├── Message       — immutable typed payload
  │   ├── Channel       — one-way append-only pipe
  │   └── ActorSystem   — lifecycle, routing, dead letters
  ```
  """

  # ============================================================================
  # Simple JSON Encoding/Decoding
  # ============================================================================
  #
  # We cannot use Jason or any external dependency. This module provides minimal
  # JSON encoding and decoding sufficient for the envelope format used in the
  # wire protocol. The envelope contains: id (string), timestamp (integer),
  # sender_id (string), content_type (string), metadata (map of string to string).
  #
  # This is NOT a general-purpose JSON library. It handles the specific shapes
  # that our envelope uses: strings, integers, maps with string keys/values,
  # and null.

  defmodule JSON do
    @moduledoc false

    @doc """
    Encode an Elixir term to a JSON string.

    Supports: strings, integers, floats, booleans, nil, lists, and maps.
    Maps must have string keys. This is sufficient for our envelope format.
    """
    def encode(nil), do: "null"
    def encode(true), do: "true"
    def encode(false), do: "false"
    def encode(value) when is_integer(value), do: Integer.to_string(value)
    def encode(value) when is_float(value), do: Float.to_string(value)

    def encode(value) when is_binary(value) do
      # Escape special characters in string values
      escaped =
        value
        |> String.replace("\\", "\\\\")
        |> String.replace("\"", "\\\"")
        |> String.replace("\n", "\\n")
        |> String.replace("\r", "\\r")
        |> String.replace("\t", "\\t")

      "\"#{escaped}\""
    end

    def encode(value) when is_list(value) do
      items = Enum.map(value, &encode/1) |> Enum.join(",")
      "[#{items}]"
    end

    def encode(value) when is_map(value) do
      pairs =
        value
        |> Enum.sort_by(fn {k, _v} -> k end)
        |> Enum.map(fn {k, v} -> "#{encode(to_string(k))}:#{encode(v)}" end)
        |> Enum.join(",")

      "{#{pairs}}"
    end

    @doc """
    Decode a JSON string into an Elixir term.

    Returns {:ok, term} on success, {:error, reason} on failure.
    """
    def decode(json_string) when is_binary(json_string) do
      json_string = String.trim(json_string)

      case parse(json_string) do
        {value, rest} ->
          if String.trim(rest) == "" do
            {:ok, value}
          else
            {:error, "unexpected trailing characters: #{inspect(rest)}"}
          end

        :error ->
          {:error, "invalid JSON"}
      end
    end

    # --- Parser internals ---

    defp parse(<<"\"", rest::binary>>), do: parse_string(rest, "")
    defp parse(<<"null", rest::binary>>), do: {nil, rest}
    defp parse(<<"true", rest::binary>>), do: {true, rest}
    defp parse(<<"false", rest::binary>>), do: {false, rest}
    defp parse(<<"{", rest::binary>>), do: parse_object(String.trim_leading(rest), %{})
    defp parse(<<"[", rest::binary>>), do: parse_array(String.trim_leading(rest), [])

    defp parse(<<c, _rest::binary>> = input) when c in [?-, ?0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9] do
      parse_number(input)
    end

    defp parse(_), do: :error

    defp parse_string(<<"\\\"", rest::binary>>, acc), do: parse_string(rest, acc <> "\"")
    defp parse_string(<<"\\\\", rest::binary>>, acc), do: parse_string(rest, acc <> "\\")
    defp parse_string(<<"\\n", rest::binary>>, acc), do: parse_string(rest, acc <> "\n")
    defp parse_string(<<"\\r", rest::binary>>, acc), do: parse_string(rest, acc <> "\r")
    defp parse_string(<<"\\t", rest::binary>>, acc), do: parse_string(rest, acc <> "\t")
    defp parse_string(<<"\\\/", rest::binary>>, acc), do: parse_string(rest, acc <> "/")

    defp parse_string(<<"\"", rest::binary>>, acc), do: {acc, rest}
    defp parse_string(<<c, rest::binary>>, acc), do: parse_string(rest, acc <> <<c>>)
    defp parse_string("", _acc), do: :error

    defp parse_number(input) do
      {num_str, leftover} = take_number_chars(input, "")

      cond do
        String.contains?(num_str, ".") or String.contains?(num_str, "e") or
            String.contains?(num_str, "E") ->
          case Float.parse(num_str) do
            {f, ""} -> {f, leftover}
            _ -> :error
          end

        true ->
          case Integer.parse(num_str) do
            {i, ""} -> {i, leftover}
            _ -> :error
          end
      end
    end

    defp take_number_chars(<<c, rest::binary>>, acc)
         when c in [?0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?., ?-, ?+, ?e, ?E] do
      take_number_chars(rest, acc <> <<c>>)
    end

    defp take_number_chars(remaining, acc), do: {acc, remaining}

    defp parse_object(<<"}", rest::binary>>, acc), do: {acc, rest}

    defp parse_object(<<",", rest::binary>>, acc) do
      parse_object(String.trim_leading(rest), acc)
    end

    defp parse_object(input, acc) do
      case parse(input) do
        {key, after_key} when is_binary(key) ->
          after_key = String.trim_leading(after_key)

          case after_key do
            <<":", value_rest::binary>> ->
              value_rest = String.trim_leading(value_rest)

              case parse(value_rest) do
                {value, after_value} ->
                  after_value = String.trim_leading(after_value)
                  parse_object(after_value, Map.put(acc, key, value))

                :error ->
                  :error
              end

            _ ->
              :error
          end

        _ ->
          :error
      end
    end

    defp parse_array(<<"]", rest::binary>>, acc), do: {Enum.reverse(acc), rest}

    defp parse_array(<<",", rest::binary>>, acc) do
      parse_array(String.trim_leading(rest), acc)
    end

    defp parse_array(input, acc) do
      case parse(input) do
        {value, after_value} ->
          after_value = String.trim_leading(after_value)
          parse_array(after_value, [value | acc])

        :error ->
          :error
      end
    end
  end

  # ============================================================================
  # Message — The Atom of Communication
  # ============================================================================
  #
  # A Message is a sealed letter. Once created, it cannot be modified. The
  # envelope records who sent it, when, and what kind of data it contains.
  # The payload is always raw bytes — the content_type field tells the receiver
  # how to interpret them (UTF-8 text, JSON, PNG image, etc.).
  #
  # ## Wire Format
  #
  # Messages serialize to a binary wire format that separates the envelope
  # (metadata, always JSON) from the payload (always raw bytes). This avoids
  # Base64-encoding binary data, which would bloat size by 33%.
  #
  # ```
  # ┌─────────────────────────────────────────────┐
  # │ HEADER (17 bytes, fixed size)               │
  # │ magic:          4 bytes  "ACTM"             │
  # │ version:        1 byte   0x01               │
  # │ envelope_length: 4 bytes (big-endian u32)   │
  # │ payload_length:  8 bytes (big-endian u64)   │
  # ├─────────────────────────────────────────────┤
  # │ ENVELOPE (UTF-8 JSON, variable length)      │
  # ├─────────────────────────────────────────────┤
  # │ PAYLOAD (raw bytes, variable length)        │
  # └─────────────────────────────────────────────┘
  # ```

  defmodule Message do
    @moduledoc """
    Immutable message — the atom of actor communication.

    A Message carries data between actors. It is immutable by design: all fields
    are set at creation time and cannot be changed. Elixir's data structures are
    immutable by nature, so this property is enforced by the language itself.

    ## Fields

    | Field        | Type    | Description                                    |
    |--------------|---------|------------------------------------------------|
    | id           | string  | Unique identifier, auto-generated at creation  |
    | timestamp    | integer | Monotonic nanosecond counter for ordering       |
    | sender_id    | string  | The actor that created this message             |
    | content_type | string  | MIME type describing the payload format         |
    | payload      | binary  | Raw bytes — content_type says how to read them  |
    | metadata     | map     | Optional key-value pairs for extensibility      |

    ## Convenience Constructors

        # Text message — payload stored as UTF-8 bytes
        Message.text("agent", "hello world")

        # JSON message — payload is JSON-serialized to UTF-8 bytes
        Message.json("agent", %{"key" => "value"})

        # Binary message — payload is raw bytes with a custom content type
        Message.binary("browser", "image/png", png_bytes)
    """

    @enforce_keys [:id, :timestamp, :sender_id, :content_type, :payload, :metadata]
    defstruct [:id, :timestamp, :sender_id, :content_type, :payload, :metadata]

    # The magic bytes that identify our wire format. Every serialized message
    # starts with these 4 bytes. If you see "ACTM" at the beginning of a file,
    # it's one of our messages.
    @wire_magic "ACTM"

    # The wire format version. Bump this when the serialization format changes.
    # Readers must handle all versions <= their own. Readers that encounter a
    # version > their own must return a clear error (not crash).
    @wire_version 1

    # A monotonic counter used to generate strictly increasing timestamps.
    # In a real system this would be per-actor, but for our single-threaded
    # implementation a global counter works. We use the process dictionary
    # to maintain state across calls since this is a pure functional module.

    @doc """
    Create a new Message with the given fields.

    The `id` and `timestamp` are auto-generated. The `id` is a unique string
    based on random bytes. The `timestamp` is a monotonic counter that strictly
    increases with each message created in this process.

    ## Parameters

    - `sender_id` — the actor that created this message (string)
    - `content_type` — MIME type describing the payload (string)
    - `payload` — the message body as raw bytes (binary)
    - `metadata` — optional key-value pairs (map, default %{})

    ## Examples

        iex> msg = Message.new("actor_1", "text/plain", "hello")
        iex> msg.sender_id
        "actor_1"
        iex> msg.content_type
        "text/plain"
    """
    def new(sender_id, content_type, payload, metadata \\ %{}) do
      %__MODULE__{
        id: generate_id(),
        timestamp: next_timestamp(),
        sender_id: sender_id,
        content_type: content_type,
        payload: payload,
        metadata: metadata
      }
    end

    @doc """
    Create a text message. Sets content_type to "text/plain" and encodes
    the payload string as UTF-8 bytes.

    ## Examples

        iex> msg = Message.text("agent", "hello world")
        iex> msg.content_type
        "text/plain"
        iex> msg.payload
        "hello world"
    """
    def text(sender_id, payload_text, metadata \\ %{}) do
      new(sender_id, "text/plain", payload_text, metadata)
    end

    @doc """
    Create a JSON message. Sets content_type to "application/json" and
    serializes the payload term to a JSON string.

    The payload can be a map or a list — anything our JSON encoder handles.

    ## Examples

        iex> msg = Message.json("agent", %{"key" => "value"})
        iex> msg.content_type
        "application/json"
    """
    def json(sender_id, payload_term, metadata \\ %{}) do
      json_str = CodingAdventures.Actor.JSON.encode(payload_term)
      new(sender_id, "application/json", json_str, metadata)
    end

    @doc """
    Create a binary message with a custom content type.

    Use this for images, videos, or any arbitrary binary data where you
    need to specify the content type explicitly.

    ## Examples

        iex> msg = Message.binary("browser", "image/png", <<137, 80, 78, 71>>)
        iex> msg.content_type
        "image/png"
    """
    def binary(sender_id, content_type, payload_bytes, metadata \\ %{}) do
      new(sender_id, content_type, payload_bytes, metadata)
    end

    @doc """
    Return the payload decoded as a UTF-8 string.

    This is a convenience for text messages. If the payload is not valid
    UTF-8, this will return the raw binary (Elixir binaries are UTF-8
    strings when they contain valid UTF-8).
    """
    def payload_text(%__MODULE__{payload: payload}), do: payload

    @doc """
    Return the payload parsed as JSON.

    This is a convenience for JSON messages. Returns {:ok, term} on success
    or {:error, reason} on failure.
    """
    def payload_json(%__MODULE__{payload: payload}) do
      CodingAdventures.Actor.JSON.decode(payload)
    end

    @doc """
    Serialize ONLY the envelope (all fields except payload) to a JSON string.

    Useful for logging, indexing, and debugging without touching the payload.
    You can scan a channel of 10GB video messages by reading only envelopes.

    ## Example

        iex> msg = Message.text("agent", "hello")
        iex> envelope = Message.envelope_to_json(msg)
        iex> String.contains?(envelope, "agent")
        true
    """
    def envelope_to_json(%__MODULE__{} = msg) do
      envelope = %{
        "id" => msg.id,
        "timestamp" => msg.timestamp,
        "sender_id" => msg.sender_id,
        "content_type" => msg.content_type,
        "metadata" => msg.metadata
      }

      CodingAdventures.Actor.JSON.encode(envelope)
    end

    @doc """
    Serialize a message to the binary wire format.

    The wire format consists of:
    1. A 17-byte fixed header: magic (4) + version (1) + envelope_length (4) + payload_length (8)
    2. The envelope as UTF-8 JSON (variable length)
    3. The raw payload bytes (variable length)

    This format is binary-native: a 10MB image is 10MB on disk, not 13.3MB
    after Base64 encoding.
    """
    def to_bytes(%__MODULE__{} = msg) do
      envelope_json = envelope_to_json(msg)
      envelope_bytes = envelope_json
      payload_bytes = msg.payload
      envelope_len = byte_size(envelope_bytes)
      payload_len = byte_size(payload_bytes)

      # Build the 17-byte header using Elixir's binary pattern matching.
      # This is where Elixir truly shines — binary construction is a
      # first-class language feature, not a library call.
      header =
        <<"ACTM", @wire_version::8, envelope_len::32-big,
          payload_len::64-big>>

      header <> envelope_bytes <> payload_bytes
    end

    @doc """
    Deserialize a message from the binary wire format.

    Returns {:ok, message} on success.
    Returns {:error, :invalid_format} if the magic bytes don't match.
    Returns {:error, {:unsupported_version, version}} if the version is
    higher than what we support.

    ## How it works

    1. Read the 17-byte header to get magic, version, envelope_length, payload_length.
    2. Validate the magic bytes ("ACTM").
    3. Check the version is supported (<= @wire_version).
    4. Read envelope_length bytes and parse as JSON.
    5. Read payload_length bytes as raw binary.
    6. Reconstruct the Message struct.
    """
    def from_bytes(data) when is_binary(data) do
      case data do
        <<"ACTM", version::8, envelope_len::32-big, payload_len::64-big,
          envelope_bytes::binary-size(envelope_len), payload_bytes::binary-size(payload_len),
          _rest::binary>> ->
          if version > @wire_version do
            {:error, {:unsupported_version, version}}
          else
            case CodingAdventures.Actor.JSON.decode(envelope_bytes) do
              {:ok, envelope} ->
                msg = %__MODULE__{
                  id: Map.get(envelope, "id"),
                  timestamp: Map.get(envelope, "timestamp"),
                  sender_id: Map.get(envelope, "sender_id"),
                  content_type: Map.get(envelope, "content_type"),
                  payload: payload_bytes,
                  metadata: Map.get(envelope, "metadata", %{})
                }

                {:ok, msg}

              {:error, reason} ->
                {:error, {:envelope_parse_error, reason}}
            end
          end

        <<"ACTM", version::8, _rest::binary>> ->
          # We matched ACTM but couldn't parse full message above.
          # Check if it's a version issue.
          if version > @wire_version do
            {:error, {:unsupported_version, version}}
          else
            {:error, :invalid_format}
          end

        <<_magic::binary-size(4), _rest::binary>> ->
          {:error, :invalid_format}

        _ ->
          {:error, :invalid_format}
      end
    end

    @doc """
    Read one message from a binary stream (IO device).

    Reads the 17-byte header first, then the envelope, then the payload.
    Returns {:ok, message} on success, :eof on end of file, or
    {:error, reason} on failure.

    This is used for channel recovery: reading messages sequentially from
    a persisted channel log file.
    """
    def from_io_device(device) do
      # Read the 17-byte header
      case IO.binread(device, 17) do
        :eof ->
          :eof

        {:error, reason} ->
          {:error, reason}

        header_data when is_binary(header_data) and byte_size(header_data) < 17 ->
          # Truncated header — partial write before crash
          {:error, :truncated}

        <<"ACTM", version::8, envelope_len::32-big, payload_len::64-big>> ->
          if version > @wire_version do
            {:error, {:unsupported_version, version}}
          else
            # Read envelope
            case IO.binread(device, envelope_len) do
              :eof ->
                {:error, :truncated}

              {:error, reason} ->
                {:error, reason}

              envelope_bytes when is_binary(envelope_bytes) and byte_size(envelope_bytes) < envelope_len ->
                {:error, :truncated}

              envelope_bytes ->
                # Read payload
                case read_payload(device, payload_len) do
                  {:ok, payload_bytes} ->
                    case CodingAdventures.Actor.JSON.decode(envelope_bytes) do
                      {:ok, envelope} ->
                        msg = %__MODULE__{
                          id: Map.get(envelope, "id"),
                          timestamp: Map.get(envelope, "timestamp"),
                          sender_id: Map.get(envelope, "sender_id"),
                          content_type: Map.get(envelope, "content_type"),
                          payload: payload_bytes,
                          metadata: Map.get(envelope, "metadata", %{})
                        }

                        {:ok, msg}

                      {:error, reason} ->
                        {:error, {:envelope_parse_error, reason}}
                    end

                  {:error, reason} ->
                    {:error, reason}
                end
            end
          end

        _other_header ->
          {:error, :invalid_format}
      end
    end

    # Read payload bytes, handling zero-length payloads
    defp read_payload(_device, 0), do: {:ok, ""}

    defp read_payload(device, payload_len) do
      case IO.binread(device, payload_len) do
        :eof -> {:error, :truncated}
        {:error, reason} -> {:error, reason}

        data when is_binary(data) and byte_size(data) < payload_len ->
          {:error, :truncated}

        data ->
          {:ok, data}
      end
    end

    @doc "Return the wire format version constant."
    def wire_version, do: @wire_version

    @doc "Return the wire format magic bytes."
    def wire_magic, do: @wire_magic

    # --- Private helpers ---

    # Generate a unique message ID using random bytes.
    # Format: "msg_" followed by 16 hex characters (8 random bytes).
    # This gives us 2^64 possible IDs — more than enough to avoid collisions
    # in any practical scenario.
    defp generate_id do
      bytes = :crypto.strong_rand_bytes(8)
      hex = Base.encode16(bytes, case: :lower)
      "msg_#{hex}"
    end

    # Generate a strictly increasing timestamp using the process dictionary.
    # Each call returns a value one greater than the previous. This ensures
    # messages created in sequence have strictly increasing timestamps,
    # which is essential for ordering.
    defp next_timestamp do
      current = Process.get(:actor_msg_timestamp, 0)
      next_val = current + 1
      Process.put(:actor_msg_timestamp, next_val)
      next_val
    end
  end

  # ============================================================================
  # Channel — One-Way, Append-Only Message Log
  # ============================================================================
  #
  # A Channel is a one-way, append-only, ordered log of messages. It connects
  # producers to consumers. Messages flow in one direction. Once appended, a
  # message cannot be removed, modified, or reordered.
  #
  # **Analogy:** A one-way pneumatic tube in an office building. Documents go
  # in one end and come out the other. The tube keeps a copy of every document
  # that has ever passed through it (the log).
  #
  # ## Why One-Way?
  #
  # Bidirectional channels create ambiguity: "who sent this message?" One-way
  # channels eliminate the question. For bidirectional communication, use two
  # channels — one in each direction.
  #
  # ## Why Append-Only?
  #
  # If messages could be deleted, crash recovery becomes impossible. With an
  # append-only log, the answer to "what happened before the crash?" is always
  # definitive: "here is exactly what happened, in order."
  #
  # ## Persistence
  #
  # Channels persist to disk using the same binary wire format as individual
  # messages. Each message is written as header + envelope + payload,
  # concatenated end-to-end. Recovery reads them back sequentially.

  defmodule Channel do
    @moduledoc """
    One-way, append-only, ordered message log.

    A Channel stores messages in order and supports reading from any offset.
    Multiple consumers can read the same channel independently at different
    offsets. Messages are never removed from the log.

    Since Elixir data is immutable, channel operations return new channel
    structs rather than modifying in place.

    ## Fields

    | Field      | Type    | Description                          |
    |------------|---------|--------------------------------------|
    | id         | string  | Unique identifier                    |
    | name       | string  | Human-readable name for discovery    |
    | log        | list    | Ordered list of messages (append-only)|
    | created_at | integer | Timestamp when channel was created   |
    """

    @enforce_keys [:id, :name, :log, :created_at]
    defstruct [:id, :name, :log, :created_at]

    @doc """
    Create a new empty Channel.

    ## Examples

        iex> ch = Channel.new("ch_001", "greetings")
        iex> ch.name
        "greetings"
        iex> Channel.length(ch)
        0
    """
    def new(channel_id, name) do
      %__MODULE__{
        id: channel_id,
        name: name,
        log: [],
        created_at: System.monotonic_time(:nanosecond)
      }
    end

    @doc """
    Append a message to the channel log.

    Returns a tuple {new_channel, sequence_number} where sequence_number
    is the 0-based index of the appended message. This is the ONLY write
    operation — there is no delete, no update, no insert-at-position.

    Since Elixir is functional, this returns a NEW channel struct with the
    message appended. The original channel is unchanged.

    ## Examples

        iex> ch = Channel.new("ch_001", "test")
        iex> msg = Message.text("agent", "hello")
        iex> {ch, seq} = Channel.append(ch, msg)
        iex> seq
        0
    """
    def append(%__MODULE__{log: log} = channel, %Message{} = msg) do
      seq = Kernel.length(log)
      new_channel = %{channel | log: log ++ [msg]}
      {new_channel, seq}
    end

    @doc """
    Read messages from the channel starting at `offset`, returning up to
    `limit` messages.

    - If offset >= log length, returns [] (caller is caught up).
    - If offset + limit > log length, returns remaining messages.
    - This does NOT consume messages — they remain in the log.

    ## Examples

        iex> ch = Channel.new("ch_001", "test")
        iex> msg = Message.text("agent", "hello")
        iex> {ch, _} = Channel.append(ch, msg)
        iex> Channel.read(ch, 0, 10)
        [msg]
    """
    def read(%__MODULE__{log: log}, offset \\ 0, limit \\ 100) do
      log
      |> Enum.drop(offset)
      |> Enum.take(limit)
    end

    @doc """
    Return the number of messages in the channel log.
    """
    def length(%__MODULE__{log: log}), do: Kernel.length(log)

    @doc """
    Return messages from index `start_idx` to `end_idx` (exclusive).

    Equivalent to `read(channel, start_idx, end_idx - start_idx)`.

    Named `channel_slice` to avoid conflict with any reserved words.

    ## Examples

        iex> # After appending 5 messages...
        iex> Channel.channel_slice(ch, 1, 4)
        # Returns messages at indices 1, 2, 3
    """
    def channel_slice(%__MODULE__{log: log}, start_idx, end_idx) do
      count = max(end_idx - start_idx, 0)

      log
      |> Enum.drop(start_idx)
      |> Enum.take(count)
    end

    @doc """
    Persist the channel log to disk as a binary append log.

    Each message is written using the wire format (header + envelope + payload).
    The file is created at `directory/channel_name.log`.

    ## Wire format on disk

    ```
    [ACTM][v1][env_len][pay_len] message 0 header
    {envelope JSON}              message 0 envelope
    <raw payload bytes>          message 0 payload
    [ACTM][v1][env_len][pay_len] message 1 header
    ...
    ```
    """
    def persist(%__MODULE__{name: name, log: log}, directory) do
      File.mkdir_p!(directory)
      path = Path.join(directory, "#{name}.log")

      binary_data =
        log
        |> Enum.map(&Message.to_bytes/1)
        |> Enum.join()

      File.write!(path, binary_data)
    end

    @doc """
    Recover a channel from a persisted log file on disk.

    Reads messages sequentially from the file using the wire format.
    If the file does not exist, returns an empty channel.
    If the file contains a truncated message at the end (partial write
    before crash), that message is discarded and all complete messages
    are recovered.

    ## Examples

        iex> Channel.persist(ch, "/tmp/channels")
        iex> recovered = Channel.recover("/tmp/channels", "greetings")
        iex> Channel.length(recovered) == Channel.length(ch)
        true
    """
    def recover(directory, name) do
      path = Path.join(directory, "#{name}.log")

      if File.exists?(path) do
        {:ok, data} = File.read(path)
        messages = read_messages_from_binary(data, [])

        %__MODULE__{
          id: "recovered_#{name}",
          name: name,
          log: messages,
          created_at: System.monotonic_time(:nanosecond)
        }
      else
        new("recovered_#{name}", name)
      end
    end

    # Read messages from a binary blob, handling truncated data gracefully.
    # This is the recovery algorithm: parse header -> read envelope -> read payload
    # -> repeat until we run out of data or hit a truncated message.
    defp read_messages_from_binary(data, acc) when byte_size(data) < 17 do
      # Not enough bytes for a header — either EOF or truncated
      Enum.reverse(acc)
    end

    defp read_messages_from_binary(data, acc) do
      case Message.from_bytes(data) do
        {:ok, msg} ->
          # Calculate how many bytes this message consumed
          envelope_json = Message.envelope_to_json(msg)
          consumed = 17 + byte_size(envelope_json) + byte_size(msg.payload)
          remaining = binary_part(data, consumed, byte_size(data) - consumed)
          read_messages_from_binary(remaining, [msg | acc])

        {:error, _reason} ->
          # Truncated or corrupt — discard and return what we have
          Enum.reverse(acc)
      end
    end
  end

  # ============================================================================
  # ActorResult — Return Value from a Behavior Function
  # ============================================================================
  #
  # When an actor's behavior function processes a message, it returns an
  # ActorResult that tells the system what happened:
  #
  # - new_state: the actor's updated state
  # - messages_to_send: list of {target_id, message} pairs to deliver
  # - actors_to_create: list of ActorSpec structs for new actors to spawn
  # - stop_actor: if true, the actor halts permanently
  #
  # Note: we use `stop_actor` instead of `stop` to avoid any ambiguity with
  # Elixir's built-in functions.

  defmodule ActorResult do
    @moduledoc """
    Return value from an actor's behavior function.

    The behavior function `fn(state, message) -> ActorResult` returns this
    struct to communicate what happened:

    - `new_state` — the actor's state after processing
    - `messages_to_send` — list of `{target_id, message}` pairs to deliver
    - `actors_to_create` — list of `ActorSpec` structs for new actors
    - `stop_actor` — if true, the actor halts permanently

    ## Examples

        # Simple state update, no messages
        %ActorResult{new_state: count + 1}

        # Reply to sender
        %ActorResult{
          new_state: state,
          messages_to_send: [{"sender_id", reply_msg}]
        }

        # Stop the actor
        %ActorResult{new_state: state, stop_actor: true}
    """

    defstruct new_state: nil,
              messages_to_send: [],
              actors_to_create: [],
              stop_actor: false
  end

  # ============================================================================
  # ActorSpec — Specification for Creating a New Actor
  # ============================================================================

  defmodule ActorSpec do
    @moduledoc """
    Specification for creating a new actor.

    Used in `ActorResult.actors_to_create` to tell the system to spawn
    new actors. Contains the new actor's ID, initial state, and behavior
    function.
    """

    @enforce_keys [:actor_id, :initial_state, :behavior]
    defstruct [:actor_id, :initial_state, :behavior]
  end

  # ============================================================================
  # Actor — Isolated Unit of Computation
  # ============================================================================
  #
  # An actor is a person in a soundproofed room with a mail slot. Letters
  # (messages) come in through the slot and pile up in a tray (mailbox).
  # The person reads one letter at a time, thinks about it, possibly writes
  # reply letters and slides them out, and possibly rearranges things on
  # their desk (state). They never leave the room. They never look into
  # anyone else's room.

  defmodule ActorState do
    @moduledoc """
    Internal representation of an actor.

    This struct is managed by the ActorSystem — user code interacts with
    actors through the system, not directly.

    ## Fields

    | Field    | Type     | Description                                |
    |----------|----------|--------------------------------------------|
    | id       | string   | Unique address for this actor              |
    | mailbox  | :queue   | FIFO queue of incoming messages             |
    | state    | any      | Private data only this actor can access    |
    | behavior | function | fn(state, message) -> ActorResult          |
    | status   | atom     | :idle, :processing, or :stopped            |
    """

    @enforce_keys [:id, :mailbox, :state, :behavior, :status]
    defstruct [:id, :mailbox, :state, :behavior, :status]
  end

  # ============================================================================
  # ActorSystem — The Runtime
  # ============================================================================
  #
  # The ActorSystem is the office building. It has a directory (which actors
  # exist), a mail room (message routing), and keeps track of undeliverable
  # mail (dead letters). Since Elixir data is immutable, all operations
  # return a new system struct.

  defmodule ActorSystem do
    @moduledoc """
    Runtime for managing actors, message delivery, and channels.

    The ActorSystem is a pure functional data structure. Every operation
    returns a new system — the original is never modified. This makes
    the system easy to test and reason about: you can take snapshots,
    compare before/after states, and replay operations deterministically.

    ## Usage

        system = ActorSystem.new()

        # Create an echo actor
        echo_behavior = fn state, msg ->
          reply = Message.text(\"echo\", \"echo: \#{Message.payload_text(msg)}\")
          %ActorResult{new_state: state, messages_to_send: [{msg.sender_id, reply}]}
        end

        system = ActorSystem.create_actor(system, \"echo\", nil, echo_behavior)

        # Send a message
        msg = Message.text(\"user\", \"hello\")
        system = ActorSystem.send_message(system, \"echo\", msg)

        # Process it
        {system, _result} = ActorSystem.process_next(system, \"echo\")
    """

    alias CodingAdventures.Actor.{Message, Channel, ActorSpec, ActorState}

    @enforce_keys [:actors, :channels, :dead_letters, :clock]
    defstruct [:actors, :channels, :dead_letters, :clock]

    @doc """
    Create a new, empty ActorSystem.

    Returns a system with no actors, no channels, no dead letters,
    and the clock set to 0.
    """
    def new do
      %__MODULE__{
        actors: %{},
        channels: %{},
        dead_letters: [],
        clock: 0
      }
    end

    # --- Actor Lifecycle ---

    @doc """
    Create a new actor and register it in the system.

    Returns {:ok, updated_system} on success.
    Returns {:error, :duplicate_id} if an actor with this ID already exists.

    The actor starts in :idle status with an empty mailbox.

    ## Parameters

    - `actor_id` — unique identifier / address for the actor
    - `initial_state` — the actor's starting state (any term)
    - `behavior` — fn(state, message) -> ActorResult

    ## Examples

        system = ActorSystem.new()
        {:ok, system} = ActorSystem.create_actor(system, "echo", nil, &echo_behavior/2)
    """
    def create_actor(%__MODULE__{} = system, actor_id, initial_state, behavior) do
      if Map.has_key?(system.actors, actor_id) do
        {:error, :duplicate_id}
      else
        actor = %ActorState{
          id: actor_id,
          mailbox: :queue.new(),
          state: initial_state,
          behavior: behavior,
          status: :idle
        }

        {:ok, %{system | actors: Map.put(system.actors, actor_id, actor)}}
      end
    end

    @doc """
    Stop an actor. Sets its status to :stopped and drains its mailbox
    to dead_letters.

    Returns the updated system. If the actor doesn't exist, returns
    the system unchanged.
    """
    def stop_actor(%__MODULE__{} = system, actor_id) do
      case Map.get(system.actors, actor_id) do
        nil ->
          system

        actor ->
          # Drain mailbox to dead letters
          drained = :queue.to_list(actor.mailbox)
          stopped_actor = %{actor | status: :stopped, mailbox: :queue.new()}

          %{
            system
            | actors: Map.put(system.actors, actor_id, stopped_actor),
              dead_letters: system.dead_letters ++ drained
          }
      end
    end

    @doc """
    Get the status of an actor.

    Returns :idle, :processing, or :stopped.
    Returns :not_found if the actor doesn't exist.
    """
    def get_actor_status(%__MODULE__{} = system, actor_id) do
      case Map.get(system.actors, actor_id) do
        nil -> :not_found
        actor -> actor.status
      end
    end

    @doc """
    Send a message to an actor's mailbox.

    If the target actor doesn't exist or is stopped, the message goes
    to dead_letters instead.

    Returns the updated system.

    Named `send_message` to avoid conflict with Kernel.send/2.
    """
    def send_message(%__MODULE__{} = system, target_id, %Message{} = msg) do
      case Map.get(system.actors, target_id) do
        nil ->
          # Actor not found — dead letter
          %{system | dead_letters: system.dead_letters ++ [msg]}

        %{status: :stopped} ->
          # Actor is stopped — dead letter
          %{system | dead_letters: system.dead_letters ++ [msg]}

        actor ->
          # Enqueue in mailbox
          new_mailbox = :queue.in(msg, actor.mailbox)
          updated_actor = %{actor | mailbox: new_mailbox}
          %{system | actors: Map.put(system.actors, target_id, updated_actor)}
      end
    end

    @doc """
    Process the next message in an actor's mailbox.

    This is the core processing loop. It:
    1. Dequeues the front message from the mailbox
    2. Sets status to :processing
    3. Calls behavior(state, message) to get an ActorResult
    4. Updates the actor's state
    5. Delivers outgoing messages
    6. Creates any new actors
    7. Sets status back to :idle (or :stopped if stop_actor is true)

    Returns {updated_system, :ok} if a message was processed.
    Returns {system, :empty} if the mailbox was empty.
    Returns {system, :not_found} if the actor doesn't exist.
    Returns {system, :stopped} if the actor is stopped.

    If the behavior function raises an exception, the message goes to
    dead_letters, the actor's state is unchanged, and it continues
    processing the next message (at-most-once semantics).
    """
    def process_next(%__MODULE__{} = system, actor_id) do
      case Map.get(system.actors, actor_id) do
        nil ->
          {system, :not_found}

        %{status: :stopped} ->
          {system, :stopped}

        actor ->
          case :queue.out(actor.mailbox) do
            {:empty, _} ->
              {system, :empty}

            {{:value, msg}, remaining_mailbox} ->
              # Set status to processing
              processing_actor = %{actor | mailbox: remaining_mailbox, status: :processing}
              system = %{system | actors: Map.put(system.actors, actor_id, processing_actor)}

              # Call the behavior function, catching exceptions
              try do
                result = processing_actor.behavior.(processing_actor.state, msg)

                # Update state and handle stop
                {updated_actor, drained} =
                  if result.stop_actor do
                    # Drain remaining mailbox to dead letters
                    drain_list = :queue.to_list(%{processing_actor | state: result.new_state}.mailbox)

                    {%{processing_actor | state: result.new_state, status: :stopped, mailbox: :queue.new()},
                     drain_list}
                  else
                    {%{processing_actor | state: result.new_state, status: :idle}, []}
                  end

                system = %{
                  system
                  | actors: Map.put(system.actors, actor_id, updated_actor),
                    dead_letters: system.dead_letters ++ drained
                }

                # Create new actors FIRST (so messages can be delivered to them)
                system =
                  Enum.reduce(result.actors_to_create, system, fn %ActorSpec{} = spec, sys ->
                    case create_actor(sys, spec.actor_id, spec.initial_state, spec.behavior) do
                      {:ok, new_sys} -> new_sys
                      {:error, _} -> sys
                    end
                  end)

                # Then deliver outgoing messages
                system =
                  Enum.reduce(result.messages_to_send, system, fn {target, outgoing_msg}, sys ->
                    send_message(sys, target, outgoing_msg)
                  end)

                {system, :ok}
              rescue
                _exception ->
                  # Behavior threw an exception:
                  # - State is unchanged (we use the original actor state)
                  # - Message goes to dead_letters
                  # - Actor continues processing (status back to :idle)
                  failed_actor = %{actor | status: :idle, mailbox: remaining_mailbox}

                  system = %{
                    system
                    | actors: Map.put(system.actors, actor_id, failed_actor),
                      dead_letters: system.dead_letters ++ [msg]
                  }

                  {system, :ok}
              end
          end
      end
    end

    @doc """
    Process all actors round-robin until no work remains.

    Finds any actor with :idle status and a non-empty mailbox, processes
    one message, then looks again. Continues until all mailboxes are empty
    or all actors are stopped.

    Returns {updated_system, stats} where stats is a map with
    :messages_processed and :actors_created counts.
    """
    def run_until_idle(%__MODULE__{} = system) do
      run_until_idle(system, %{messages_processed: 0, actors_created: 0})
    end

    defp run_until_idle(system, stats) do
      # Find an actor with work to do
      workable =
        system.actors
        |> Enum.find(fn {_id, actor} ->
          actor.status == :idle and not :queue.is_empty(actor.mailbox)
        end)

      case workable do
        nil ->
          # No work to do — system is idle
          {system, stats}

        {actor_id, _actor} ->
          actor_count_before = map_size(system.actors)
          {system, _result} = process_next(system, actor_id)
          actor_count_after = map_size(system.actors)

          stats = %{
            stats
            | messages_processed: stats.messages_processed + 1,
              actors_created: stats.actors_created + (actor_count_after - actor_count_before)
          }

          run_until_idle(system, stats)
      end
    end

    @doc """
    Like run_until_idle but keeps going until the system is fully quiet.

    In practice, for our single-threaded model, this behaves the same as
    run_until_idle since there's no concurrent message generation. But
    semantically it represents "keep processing until nothing could
    possibly generate new work."
    """
    def run_until_done(%__MODULE__{} = system) do
      {system, stats} = run_until_idle(system)

      # Check if any new work was generated
      has_work =
        Enum.any?(system.actors, fn {_id, actor} ->
          actor.status == :idle and not :queue.is_empty(actor.mailbox)
        end)

      if has_work do
        {system2, stats2} = run_until_done(system)

        combined = %{
          messages_processed: stats.messages_processed + stats2.messages_processed,
          actors_created: stats.actors_created + stats2.actors_created
        }

        {system2, combined}
      else
        {system, stats}
      end
    end

    # --- Channels ---

    @doc """
    Create and register a new channel in the system.

    Returns the updated system with the channel added.
    """
    def create_channel(%__MODULE__{} = system, channel_id, name) do
      channel = Channel.new(channel_id, name)
      %{system | channels: Map.put(system.channels, channel_id, channel)}
    end

    @doc """
    Retrieve a channel by ID.

    Returns {:ok, channel} or {:error, :not_found}.
    """
    def get_channel(%__MODULE__{} = system, channel_id) do
      case Map.get(system.channels, channel_id) do
        nil -> {:error, :not_found}
        channel -> {:ok, channel}
      end
    end

    @doc """
    Update a channel in the system (e.g., after appending a message).

    Returns the updated system.
    """
    def put_channel(%__MODULE__{} = system, channel_id, %Channel{} = channel) do
      %{system | channels: Map.put(system.channels, channel_id, channel)}
    end

    # --- Inspection ---

    @doc "List all registered actor IDs."
    def actor_ids(%__MODULE__{} = system) do
      Map.keys(system.actors)
    end

    @doc "Return the number of pending messages in an actor's mailbox."
    def mailbox_size(%__MODULE__{} = system, actor_id) do
      case Map.get(system.actors, actor_id) do
        nil -> 0
        actor -> :queue.len(actor.mailbox)
      end
    end

    @doc "Return the actor's current state (for testing/inspection)."
    def get_actor_state(%__MODULE__{} = system, actor_id) do
      case Map.get(system.actors, actor_id) do
        nil -> nil
        actor -> actor.state
      end
    end

    @doc """
    Shut down the entire system. Stops all actors, drains all mailboxes
    to dead_letters.
    """
    def shutdown(%__MODULE__{} = system) do
      Enum.reduce(Map.keys(system.actors), system, fn actor_id, sys ->
        stop_actor(sys, actor_id)
      end)
    end
  end
end
