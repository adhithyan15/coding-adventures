defmodule Rpc.Server do
  @moduledoc """
  Codec-agnostic RPC server dispatch loop.

  ## Overview

  `Rpc.Server` owns a codec module, a framer module, an initial framer state,
  and two dispatch tables (one for request handlers, one for notification
  handlers). Its `serve/4` function drives a blocking loop that:

  1. Calls `framer_module.read_frame(framer_state)` to get the next raw frame.
  2. Calls `codec_module.decode(frame)` to turn the bytes into an `Rpc.Message`.
  3. Dispatches to the appropriate handler.
  4. For requests: calls `codec_module.encode(response)` and
     `framer_module.write_frame(bytes, framer_state)` to send the response.
  5. Repeats until `:eof` or an unrecoverable I/O error.

  The server is a *plain module* (not a GenServer). `serve/4` is a blocking
  function — it returns only when the connection closes.

  ## Pluggable Codec and Framer

  The codec and framer are passed as module references. Any module that
  implements the `Rpc.Codec` behaviour can be used as the codec; any module
  that implements the `Rpc.Framer` behaviour can be used as the framer. This
  lets you swap JSON for MessagePack, or Content-Length framing for newline
  framing, without changing the handler code at all.

  ## Handler Registration

  Handlers are plain Elixir functions:

  - **Request handlers** take `(id, params)` and return either:
    - Any value → sent as `result` in the response.
    - `{:error, %Rpc.Message.ErrorResponse{}}` → sent as an error response.

  - **Notification handlers** take `(params)`. Their return value is ignored.
    Unknown notifications are silently dropped per the RPC spec.

  Handlers are stored in plain maps keyed by the method name string. The maps
  are passed as the `handlers` argument to `serve/4` (a map with two keys:
  `:request` and `:notification`). Use `register_request/3` and
  `register_notification/3` to build the handlers map.

  ## Exception Safety

  If a request handler raises an exception, the server catches it (via
  `try/rescue`) and sends an Internal Error (`-32603`) response. This keeps the
  server alive through handler bugs — a single misbehaving handler cannot kill
  the entire server process.

  Notification handler exceptions are also caught, but since notifications must
  not generate responses, the exception is silently absorbed.

  ## Usage

      # Build the handlers map.
      handlers =
        %{}
        |> Rpc.Server.register_request("ping", fn _id, _params -> "pong" end)
        |> Rpc.Server.register_notification("log", fn params ->
          IO.puts(params["message"])
        end)

      # Create the initial framer state.
      framer_state = MyFramer.new(:stdio, :stdio)

      # Block until the connection closes.
      Rpc.Server.serve(MyCodec, MyFramer, framer_state, handlers)
  """

  alias Rpc.{Errors, Message}
  alias Rpc.Message.{ErrorResponse, Notification, Request, Response}

  # ---------------------------------------------------------------------------
  # Handlers map helpers
  # ---------------------------------------------------------------------------
  #
  # The handlers map has two keys:
  #   :request      => %{method_name => fn(id, params)}
  #   :notification => %{method_name => fn(params)}
  #
  # We provide `register_request/3` and `register_notification/3` as pure
  # functions that build or update this map. This is idiomatic Elixir — no
  # mutable state, easy to compose with |>.

  @doc """
  Add a request handler to a handlers map.

  The handler function receives `(id, params)` and must return either:
  - Any value — sent as `result` in the response.
  - `{:error, %Rpc.Message.ErrorResponse{}}` — sent as an error response.

  Registering the same method twice replaces the earlier handler.

  ## Examples

      handlers = Rpc.Server.register_request(%{}, "ping", fn _id, _params -> "pong" end)
  """
  @spec register_request(map(), String.t(), (term(), term() -> term())) :: map()
  def register_request(handlers, method, fun)
      when is_map(handlers) and is_binary(method) and is_function(fun, 2) do
    request_handlers = Map.get(handlers, :request, %{})
    Map.put(handlers, :request, Map.put(request_handlers, method, fun))
  end

  @doc """
  Add a notification handler to a handlers map.

  The handler function receives `(params)`. Its return value is ignored.
  Unknown notifications are silently dropped.

  ## Examples

      handlers = Rpc.Server.register_notification(%{}, "log", fn _params -> :ok end)
  """
  @spec register_notification(map(), String.t(), (term() -> term())) :: map()
  def register_notification(handlers, method, fun)
      when is_map(handlers) and is_binary(method) and is_function(fun, 1) do
    notif_handlers = Map.get(handlers, :notification, %{})
    Map.put(handlers, :notification, Map.put(notif_handlers, method, fun))
  end

  # ---------------------------------------------------------------------------
  # serve/4 — the blocking dispatch loop
  # ---------------------------------------------------------------------------

  @doc """
  Start the blocking read-dispatch-write loop.

  Arguments:
  - `codec_module`   — A module implementing the `Rpc.Codec` behaviour.
  - `framer_module`  — A module implementing the `Rpc.Framer` behaviour.
  - `framer_state`   — The initial state for the framer (opaque term).
  - `handlers`       — A map built with `register_request/3` and
                       `register_notification/3`. Defaults to `%{}`.

  Returns `:ok` on clean EOF. On an unrecoverable I/O error the loop
  terminates and propagates the error tuple.

  The loop processes one message at a time (single-threaded). This is
  correct for most RPC use cases. Future versions may add a concurrent
  variant without changing the handler API.
  """
  @spec serve(module(), module(), term(), map()) :: :ok
  def serve(codec_module, framer_module, framer_state, handlers \\ %{}) do
    do_serve(codec_module, framer_module, framer_state, handlers)
  end

  # ---------------------------------------------------------------------------
  # Private: recursive serve loop
  # ---------------------------------------------------------------------------
  #
  # The loop steps:
  #   1. read_frame → get raw bytes or :eof or {:error, reason}
  #   2. On :eof    → return :ok (clean shutdown)
  #   3. On {:error, _} → break the loop (unrecoverable I/O problem)
  #   4. Decode the bytes → get an Rpc.Message or {:error, ErrorResponse}
  #   5. Dispatch to the appropriate private handler
  #   6. Recurse with the updated framer state

  defp do_serve(codec_module, framer_module, framer_state, handlers) do
    case framer_module.read_frame(framer_state) do
      :eof ->
        # Clean EOF — the connection was closed gracefully.
        :ok

      {:error, reason} ->
        # Unrecoverable I/O error — propagate.
        {:error, reason}

      {:ok, bytes, new_framer_state} ->
        # Decode the frame bytes into an RPC message.
        case codec_module.decode(bytes) do
          {:error, %ErrorResponse{} = error_resp} ->
            # Framing or codec decode error. Send error response with null id.
            send_response(codec_module, framer_module, new_framer_state, error_resp)
            do_serve(codec_module, framer_module, new_framer_state, handlers)

          {:ok, %Request{} = req} ->
            new_state =
              dispatch_request(codec_module, framer_module, new_framer_state, handlers, req)

            do_serve(codec_module, framer_module, new_state, handlers)

          {:ok, %Notification{} = notif} ->
            dispatch_notification(handlers, notif)
            do_serve(codec_module, framer_module, new_framer_state, handlers)

          {:ok, %Response{}} ->
            # Servers that only respond to requests ignore incoming responses.
            # (Bidirectional peers would route these to a pending-request table.)
            do_serve(codec_module, framer_module, new_framer_state, handlers)

          {:ok, %ErrorResponse{}} ->
            # Same: ignore incoming error responses in server mode.
            do_serve(codec_module, framer_module, new_framer_state, handlers)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: request dispatch
  # ---------------------------------------------------------------------------
  #
  # Steps:
  #   1. Look up the method in the :request handler table.
  #   2. If missing → send Method Not Found (-32601).
  #   3. Call the handler inside try/rescue.
  #      - Normal return value → send as success response.
  #      - {:error, %ErrorResponse{}} → send the provided error.
  #      - Exception → send Internal Error (-32603).
  #
  # We always return the updated framer state so the loop can continue.

  defp dispatch_request(codec_module, framer_module, framer_state, handlers, %Request{
         id: id,
         method: method,
         params: params
       }) do
    request_handlers = Map.get(handlers, :request, %{})

    case Map.fetch(request_handlers, method) do
      :error ->
        # No handler registered → Method Not Found.
        error_resp = %ErrorResponse{
          id: id,
          code: Errors.method_not_found(),
          message: "Method not found",
          data: method
        }

        send_response(codec_module, framer_module, framer_state, error_resp)

      {:ok, handler} ->
        result =
          try do
            handler.(id, params)
          rescue
            e ->
              # Handler raised an exception — catch it and return an
              # internal-error ErrorResponse so the server survives.
              {:__rpc_internal_error__,
               %ErrorResponse{
                 id: id,
                 code: Errors.internal_error(),
                 message: "Internal error",
                 data: Exception.message(e)
               }}
          end

        case result do
          {:__rpc_internal_error__, %ErrorResponse{} = err} ->
            send_response(codec_module, framer_module, framer_state, err)

          {:error, %ErrorResponse{} = err} ->
            # Handler signalled an application-level error.
            # Ensure the error response carries the correct request id.
            send_response(codec_module, framer_module, framer_state, %{err | id: id})

          _ ->
            # Anything else is a successful result.
            success_resp = %Response{id: id, result: result}
            send_response(codec_module, framer_module, framer_state, success_resp)
        end
    end

    # Return the framer state unchanged — writes are side effects on the framer
    # device, not changes to the framer state structure in our simple design.
    framer_state
  end

  # ---------------------------------------------------------------------------
  # Private: notification dispatch
  # ---------------------------------------------------------------------------
  #
  # Notifications never generate responses — even if the handler raises.
  # Unknown notifications are silently dropped per the RPC spec.

  defp dispatch_notification(handlers, %Notification{method: method, params: params}) do
    notif_handlers = Map.get(handlers, :notification, %{})

    case Map.fetch(notif_handlers, method) do
      :error ->
        # Unknown notification — silently ignored.
        :ok

      {:ok, handler} ->
        try do
          handler.(params)
        rescue
          # Exception in notification handler — absorb silently.
          # Notifications must not generate error responses.
          _e -> :ok
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: response writer
  # ---------------------------------------------------------------------------
  #
  # Encodes the response message with the codec and writes it with the framer.
  # Encode or write failures are currently logged and swallowed — crashing the
  # serve loop for a single bad response would be worse than silently dropping
  # it.

  defp send_response(codec_module, framer_module, framer_state, msg) do
    case codec_module.encode(msg) do
      {:ok, bytes} ->
        case framer_module.write_frame(bytes, framer_state) do
          {:ok, _new_state} -> :ok
          {:error, reason} -> {:error, reason}
        end

      {:error, reason} ->
        # Encoding failed — this is an internal bug in the codec, not a
        # handler error. Log and continue.
        require Logger
        Logger.error("Rpc.Server: failed to encode response: #{inspect(reason)}")
        :ok
    end
  end
end
