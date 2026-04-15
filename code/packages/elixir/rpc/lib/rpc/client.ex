defmodule Rpc.Client do
  @moduledoc """
  Codec-agnostic RPC client for sending requests and notifications.

  ## Overview

  `Rpc.Client` wraps a codec module and a framer to provide a simple blocking
  API for calling remote procedures:

  - `request/3` — send a request, block until the matching response arrives,
    return the result or an error.
  - `notify/3` — fire-and-forget; no response is expected.

  ## Synchronous (Blocking) Design

  The client uses a simple synchronous model: it sends one request, then reads
  frames until it receives a response with a matching id. While waiting it may
  receive server-push notifications, which are dispatched to registered handlers
  before continuing to wait.

  This model is appropriate for most RPC use cases (e.g., LSP editor plugins,
  CLI tools) where one request is sent at a time. For concurrent use, a separate
  reader process and a pending-request map would be needed — that is left as a
  future extension.

  ## Id Management

  The client maintains a monotonically increasing integer counter. Each call to
  `request/3` increments the counter and uses the new value as the request id.
  The counter starts at 1.

  ## Immutable State

  The client is represented as a plain struct. `request/3` and `notify/3`
  return an updated `{result, client}` tuple. This keeps the client value
  thread-safe and easy to reason about.

  ## Usage

      # Create a client.
      client = Rpc.Client.new(MyCodec, MyFramer, MyFramer.new(:stdio, :stdio))

      # Send a request and wait for the response.
      {:ok, result, client2} = Rpc.Client.request(client, "ping", nil)

      # Send a fire-and-forget notification.
      {:ok, client3} = Rpc.Client.notify(client2, "log", %{"message" => "hello"})
  """

  alias Rpc.{Errors, Message}
  alias Rpc.Message.{ErrorResponse, Notification, Request, Response}

  # ---------------------------------------------------------------------------
  # Client struct
  # ---------------------------------------------------------------------------
  #
  # Fields:
  #   codec_module   — module implementing Rpc.Codec
  #   framer_module  — module implementing Rpc.Framer
  #   framer_state   — opaque framer state (updated on each read/write)
  #   next_id        — monotonically increasing request id counter
  #   notif_handlers — map of method_name => fn(params)

  @enforce_keys [:codec_module, :framer_module, :framer_state]
  defstruct [:codec_module, :framer_module, :framer_state, next_id: 1, notif_handlers: %{}]

  @typedoc "An RPC client instance."
  @type t :: %__MODULE__{
          codec_module: module(),
          framer_module: module(),
          framer_state: term(),
          next_id: pos_integer(),
          notif_handlers: %{String.t() => (term() -> term())}
        }

  # ---------------------------------------------------------------------------
  # new/3 — construct a client
  # ---------------------------------------------------------------------------

  @doc """
  Create a new RPC client.

  Arguments:
  - `codec_module`  — A module implementing the `Rpc.Codec` behaviour.
  - `framer_module` — A module implementing the `Rpc.Framer` behaviour.
  - `framer_state`  — The initial framer state (opaque term from the framer's
                      constructor).

  ## Example

      client = Rpc.Client.new(MyCodec, MyFramer, MyFramer.new(:stdio, :stdio))
  """
  @spec new(module(), module(), term()) :: t()
  def new(codec_module, framer_module, framer_state) do
    %__MODULE__{
      codec_module: codec_module,
      framer_module: framer_module,
      framer_state: framer_state
    }
  end

  # ---------------------------------------------------------------------------
  # on_notification/3 — register a server-push notification handler
  # ---------------------------------------------------------------------------

  @doc """
  Register a handler for server-push notifications received while waiting for
  a response.

  Some RPC servers send unprompted notifications (e.g., LSP
  `textDocument/publishDiagnostics`). This handler is called whenever such a
  notification arrives during a blocking `request/3` call.

  Returns the updated client.

  ## Example

      client =
        Rpc.Client.new(MyCodec, MyFramer, state)
        |> Rpc.Client.on_notification("log", fn params ->
          IO.puts(params["message"])
        end)
  """
  @spec on_notification(t(), String.t(), (term() -> term())) :: t()
  def on_notification(%__MODULE__{} = client, method, fun)
      when is_binary(method) and is_function(fun, 1) do
    %{client | notif_handlers: Map.put(client.notif_handlers, method, fun)}
  end

  # ---------------------------------------------------------------------------
  # request/3 — send a request, wait for the matching response
  # ---------------------------------------------------------------------------

  @doc """
  Send a request and block until the matching response arrives.

  Generates the request id automatically (monotonically increasing integer).
  Encodes the request with the codec and writes it with the framer. Then reads
  frames until it receives a response with a matching id.

  While waiting, any server-push notifications received are dispatched to
  handlers registered via `on_notification/3`. Responses for other ids are
  silently ignored (this handles stale responses from a previous, already-timed-
  out request).

  Returns:
  - `{:ok, result, updated_client}` — the server responded with a result.
  - `{:error, %Rpc.Message.ErrorResponse{}, updated_client}` — the server
    responded with an error, or the connection was closed.

  ## Example

      {:ok, result, client2} = Rpc.Client.request(client, "ping", nil)
  """
  @spec request(t(), String.t(), term()) ::
          {:ok, term(), t()} | {:error, ErrorResponse.t(), t()}
  def request(%__MODULE__{} = client, method, params \\ nil) do
    id = client.next_id
    client = %{client | next_id: id + 1}

    msg = %Request{id: id, method: method, params: params}

    case encode_and_send(client, msg) do
      {:error, reason} ->
        error = %ErrorResponse{
          id: id,
          code: Errors.internal_error(),
          message: "Internal error",
          data: "failed to send request: #{inspect(reason)}"
        }

        {:error, error, client}

      {:ok, client} ->
        # Block, reading frames until we get the matching response.
        wait_for_response(client, id)
    end
  end

  # ---------------------------------------------------------------------------
  # notify/3 — send a fire-and-forget notification
  # ---------------------------------------------------------------------------

  @doc """
  Send a notification. No response is expected or waited for.

  ## Example

      {:ok, client2} = Rpc.Client.notify(client, "log", %{"message" => "hello"})
  """
  @spec notify(t(), String.t(), term()) :: {:ok, t()} | {:error, term()}
  def notify(%__MODULE__{} = client, method, params \\ nil) do
    msg = %Notification{method: method, params: params}

    case encode_and_send(client, msg) do
      {:ok, client} -> {:ok, client}
      {:error, reason} -> {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Private: encode and write a message via the framer
  # ---------------------------------------------------------------------------

  defp encode_and_send(%__MODULE__{} = client, msg) do
    case client.codec_module.encode(msg) do
      {:error, reason} ->
        {:error, reason}

      {:ok, bytes} ->
        case client.framer_module.write_frame(bytes, client.framer_state) do
          {:ok, new_framer_state} ->
            {:ok, %{client | framer_state: new_framer_state}}

          {:error, reason} ->
            {:error, reason}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: blocking response wait loop
  # ---------------------------------------------------------------------------
  #
  # Read frames until we see a Response or ErrorResponse with the matching id.
  # While waiting:
  #   - Notifications → dispatch to registered handlers and keep waiting.
  #   - Responses for other ids → ignore and keep waiting.
  #   - :eof → return a connection-closed error.
  #   - {:error, reason} → return an I/O error.

  defp wait_for_response(%__MODULE__{} = client, expected_id) do
    case client.framer_module.read_frame(client.framer_state) do
      :eof ->
        error = %ErrorResponse{
          id: expected_id,
          code: Errors.internal_error(),
          message: "Internal error",
          data: "connection closed before response arrived"
        }

        {:error, error, client}

      {:error, reason} ->
        error = %ErrorResponse{
          id: expected_id,
          code: Errors.internal_error(),
          message: "Internal error",
          data: "I/O error while waiting for response: #{inspect(reason)}"
        }

        {:error, error, client}

      {:ok, bytes, new_framer_state} ->
        client = %{client | framer_state: new_framer_state}

        case client.codec_module.decode(bytes) do
          {:ok, %Response{id: ^expected_id, result: result}} ->
            # Found the matching success response.
            {:ok, result, client}

          {:ok, %ErrorResponse{id: ^expected_id} = err} ->
            # Found the matching error response.
            {:error, err, client}

          {:ok, %Notification{method: method, params: params}} ->
            # Server-push notification received while waiting. Dispatch it
            # and keep waiting.
            case Map.fetch(client.notif_handlers, method) do
              {:ok, handler} ->
                try do
                  handler.(params)
                rescue
                  _e -> :ok
                end

              :error ->
                :ok
            end

            wait_for_response(client, expected_id)

          _ ->
            # Response for another id, or a message we don't handle in client
            # mode. Ignore and keep waiting.
            wait_for_response(client, expected_id)
        end
    end
  end
end
