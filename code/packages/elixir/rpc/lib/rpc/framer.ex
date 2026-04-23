defmodule Rpc.Framer do
  @moduledoc """
  Behaviour for splitting a raw byte stream into discrete message frames.

  ## What is Framing?

  A raw byte stream — like stdin/stdout, a TCP connection, or a Unix socket —
  is just a river of bytes. It does not come with any built-in notion of where
  one message ends and the next begins. Framing solves this problem.

  Think of framing like envelopes in a mail system. Multiple letters might be
  stuffed into the same post bag. The envelope tells the recipient where each
  letter starts and ends. The contents of the letter is the message payload;
  the envelope is the framing.

  ## Common Framing Schemes

  | Framer                | How it delimits frames                              |
  |-----------------------|-----------------------------------------------------|
  | `ContentLengthFramer` | `Content-Length: N\\r\\n\\r\\n` header (LSP/DAP)   |
  | `LengthPrefixFramer`  | 4-byte big-endian integer prefix                    |
  | `NewlineFramer`       | Newline character `\\n` after each payload          |
  | `WebSocketFramer`     | WebSocket data frame envelope                       |
  | `PassthroughFramer`   | No framing — the entire stream is one frame         |

  ## Separation of Concerns

  The framer knows nothing about the content of the payload bytes. It does not
  know whether they are JSON, MessagePack, Protobuf, or something else. The
  framer only knows how to read and write the *envelope* — where a frame starts
  and ends.

  ```
  wire ──[read_frame]──► bytes (payload only, no envelope)
  bytes (payload only) ──[write_frame]──► wire (with envelope)
  ```

  ## Stateful Design

  Unlike codecs (which are stateless), framers are stateful because parsing a
  frame often requires reading ahead or tracking leftover bytes from a previous
  read. The state is an opaque Elixir term managed by the caller.

  - `read_frame/1` receives the current state, returns the next frame's bytes
    and the updated state.
  - `write_frame/2` receives bytes and the current state, returns the updated
    state.

  The initial state is whatever `new/1` or the framer's constructor returns.
  The `Rpc.Server` and `Rpc.Client` pass this state through the loop.

  ## Implementing a Framer

  ```elixir
  defmodule MyNewlineFramer do
    @behaviour Rpc.Framer

    def new(device), do: %{device: device, buf: ""}

    @impl Rpc.Framer
    def read_frame(%{device: dev} = state) do
      case IO.read(dev, :line) do
        :eof -> :eof
        {:error, reason} -> {:error, reason}
        line -> {:ok, String.trim_trailing(line, "\\n"), state}
      end
    end

    @impl Rpc.Framer
    def write_frame(data, %{device: dev} = state) do
      IO.binwrite(dev, data <> "\\n")
      {:ok, state}
    end
  end
  ```
  """

  # ---------------------------------------------------------------------------
  # Callbacks
  # ---------------------------------------------------------------------------

  @doc """
  Read the next frame from the stream.

  Receives the current framer state (an opaque term). Blocks until a complete
  frame is available or the stream ends.

  Returns:
  - `{:ok, binary(), new_state}` — a complete payload frame (no envelope bytes
    included) and the updated framer state.
  - `:eof` — the stream was closed cleanly; no more frames will arrive.
  - `{:error, term()}` — an I/O error or a malformed frame envelope.
  """
  @callback read_frame(state :: term()) ::
              {:ok, binary(), term()} | :eof | {:error, term()}

  @doc """
  Write a frame to the stream.

  Wraps the payload `data` in whatever envelope the framing scheme requires
  (e.g., a Content-Length header) and writes the result to the underlying
  stream.

  Returns:
  - `{:ok, new_state}` — write succeeded; updated framer state.
  - `{:error, term()}` — I/O error or serialization failure.
  """
  @callback write_frame(data :: binary(), state :: term()) ::
              {:ok, term()} | {:error, term()}
end
