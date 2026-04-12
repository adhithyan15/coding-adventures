defmodule CodingAdventures.JsonRpc.Server do
  @moduledoc """
  JSON-RPC 2.0 Server — reads messages from a stream, dispatches to handlers,
  and writes responses.

  ## Architecture

  The server is a plain module (not a GenServer). `serve/1` runs a blocking
  recursive loop that:

  1. Reads one message from the `MessageReader`.
  2. Dispatches based on message type:
     - `%Request{}` → call the registered handler, send a `%Response{}`.
     - `%Notification{}` → call the registered handler (no response).
     - `%Response{}` → ignored (servers that only respond don't need this).
  3. Repeats until EOF or unrecoverable I/O error.

  ## Handler Signatures

  ```elixir
  # Request handler — receives id and params, returns result or error map
  fn id, params -> %{"capabilities" => %{}} end
  fn id, params -> %{code: -32602, message: "Invalid params"} end

  # Notification handler — receives params, return value is ignored
  fn params -> :ok end
  ```

  Handlers may return:
  - Any value → sent as `"result"` in the Response
  - A map with `:code` (integer) and `:message` (string) → sent as `"error"`

  ## Error Handling

  If a request handler raises an exception, the server catches it and sends an
  Internal Error (`-32603`) response. This keeps the server alive through
  handler bugs — a single bad request does not kill the entire server process.

  If no handler is registered for a method, the server sends `Method not found
  (-32601)` automatically.

  Notifications with no registered handler are silently ignored per the
  JSON-RPC 2.0 specification.

  ## Usage

      alias CodingAdventures.JsonRpc.{Server, Message}

      server =
        Server.new(:stdio, :stdio)
        |> Server.on_request("initialize", fn _id, _params ->
          %{"capabilities" => %{"hoverProvider" => true}}
        end)
        |> Server.on_notification("textDocument/didOpen", fn params ->
          # parse and store the document
          :ok
        end)

      Server.serve(server)   # blocks until EOF

  ## Concurrency Note

  The serve loop is single-threaded — one message is processed at a time. This
  is correct for LSP editors, which send requests one at a time and wait for
  responses (except for notifications and cancellation). A future multi-threaded
  variant can be added without changing the handler API.
  """

  alias CodingAdventures.JsonRpc.{Errors, Message, Reader, Writer}
  alias CodingAdventures.JsonRpc.Message.{Notification, Request, Response}

  # ---------------------------------------------------------------------------
  # Server struct
  # ---------------------------------------------------------------------------
  #
  # We store:
  # - reader / writer: MessageReader / MessageWriter instances
  # - request_handlers: map of method_name → fn(id, params)
  # - notification_handlers: map of method_name → fn(params)

  defstruct [:reader, :writer, request_handlers: %{}, notification_handlers: %{}]

  @type handler_fn :: (any(), any() -> any())
  @type notif_fn :: (any() -> any())

  @type t :: %__MODULE__{
          reader: Reader.t(),
          writer: Writer.t(),
          request_handlers: %{String.t() => handler_fn()},
          notification_handlers: %{String.t() => notif_fn()}
        }

  # ---------------------------------------------------------------------------
  # new/2 — create a server from two I/O devices
  # ---------------------------------------------------------------------------

  @doc """
  Create a new Server that reads from `in_device` and writes to `out_device`.

  ## Example

      server = Server.new(:stdio, :stdio)
  """
  @spec new(any(), any()) :: t()
  def new(in_device, out_device) do
    %__MODULE__{
      reader: Reader.new(in_device),
      writer: Writer.new(out_device)
    }
  end

  # ---------------------------------------------------------------------------
  # on_request/3 — register a request handler (chainable)
  # ---------------------------------------------------------------------------

  @doc """
  Register a handler function for a JSON-RPC method that expects a response.

  Returns the updated server struct so calls can be chained:

  ```elixir
  server
  |> Server.on_request("initialize", &my_init_handler/2)
  |> Server.on_request("shutdown", fn _id, _params -> nil end)
  ```

  ## Handler Contract

  The handler receives `(id, params)` and must return either:
  - Any value — sent as `result` in the response
  - A map with `:code` (integer) and `:message` (string) — sent as `error`
  """
  @spec on_request(t(), String.t(), handler_fn()) :: t()
  def on_request(%__MODULE__{} = server, method, handler)
      when is_binary(method) and is_function(handler, 2) do
    %{server | request_handlers: Map.put(server.request_handlers, method, handler)}
  end

  # ---------------------------------------------------------------------------
  # on_notification/3 — register a notification handler (chainable)
  # ---------------------------------------------------------------------------

  @doc """
  Register a handler function for a JSON-RPC notification (no response sent).

  ```elixir
  server
  |> Server.on_notification("textDocument/didOpen", fn params ->
    # do something
    :ok
  end)
  ```

  ## Handler Contract

  The handler receives `params` only (no `id` — notifications have no id).
  The return value is ignored. Exceptions are logged but do not crash the server.
  """
  @spec on_notification(t(), String.t(), notif_fn()) :: t()
  def on_notification(%__MODULE__{} = server, method, handler)
      when is_binary(method) and is_function(handler, 1) do
    %{server | notification_handlers: Map.put(server.notification_handlers, method, handler)}
  end

  # ---------------------------------------------------------------------------
  # serve/1 — blocking dispatch loop
  # ---------------------------------------------------------------------------

  @doc """
  Start the read-dispatch-write loop. Blocks until EOF on the input device.

  For each message:
  - `%Request{}` → dispatch to handler, write response (or error).
  - `%Notification{}` → dispatch to handler, write nothing.
  - `%Response{}` → ignored.
  - parse error → send an error response with null id.
  """
  @spec serve(t()) :: :ok
  def serve(%__MODULE__{} = server) do
    case Reader.read_message(server.reader) do
      {:ok, nil} ->
        # EOF — clean shutdown.
        :ok

      {:ok, %Request{} = req} ->
        handle_request(server, req)
        serve(server)

      {:ok, %Notification{} = notif} ->
        handle_notification(server, notif)
        serve(server)

      {:ok, %Response{}} ->
        # Responses are for client-side usage; servers ignore them.
        serve(server)

      {:error, error_map} ->
        # Framing or parse error — send an error response with null id and
        # keep serving (do not crash).
        send_error_response(server.writer, nil, error_map)
        serve(server)
    end
  end

  # ---------------------------------------------------------------------------
  # Private: request dispatch
  # ---------------------------------------------------------------------------
  #
  # Look up the method in the request_handlers map:
  # - Found: call the handler, send result or error response.
  # - Not found: send Method not found (-32601).
  # - Handler raises: send Internal error (-32603).

  defp handle_request(server, %Request{id: id, method: method, params: params}) do
    case Map.fetch(server.request_handlers, method) do
      :error ->
        send_error_response(server.writer, id, Errors.make_method_not_found(method))

      {:ok, handler} ->
        result =
          try do
            handler.(id, params)
          rescue
            e ->
              # Convert the exception into an internal error response so the
              # server survives bad handlers.
              {:__json_rpc_error__, Errors.make_internal_error(Exception.message(e))}
          end

        case result do
          {:__json_rpc_error__, error_map} ->
            send_error_response(server.writer, id, error_map)

          %{code: code, message: msg} when is_integer(code) and is_binary(msg) ->
            # Handler returned a ResponseError map — send it as an error.
            send_error_response(server.writer, id, result)

          _ ->
            # Handler returned a normal result value.
            send_success_response(server.writer, id, result)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: notification dispatch
  # ---------------------------------------------------------------------------
  #
  # Look up the method in the notification_handlers map:
  # - Found: call the handler. Ignore the return value.
  # - Not found: silently ignore (per JSON-RPC spec).
  # - Handler raises: log and continue (no error response for notifications).

  defp handle_notification(server, %Notification{method: method, params: params}) do
    case Map.fetch(server.notification_handlers, method) do
      :error ->
        # Per JSON-RPC spec, unknown notifications are silently dropped.
        :ok

      {:ok, handler} ->
        try do
          handler.(params)
        rescue
          _e ->
            # Notification handlers cannot return errors — we silently absorb.
            :ok
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Private: response helpers
  # ---------------------------------------------------------------------------

  defp send_success_response(writer, id, result) do
    response = %Response{id: id, result: result}
    Writer.write_message(writer, response)
  end

  defp send_error_response(writer, id, error_map) do
    response = %Response{id: id, error: error_map}
    Writer.write_message(writer, response)
  end
end
