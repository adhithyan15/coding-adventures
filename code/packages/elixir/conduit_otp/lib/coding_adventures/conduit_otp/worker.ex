defmodule CodingAdventures.ConduitOtp.Worker do
  @moduledoc """
  Teaching topic: `GenServer` lifecycle, cooperative looping, and "let it crash".

  ## What this is

  Each `Worker` handles exactly one TCP connection. It is started by the
  `WorkerSupervisor` and lives only as long as the connection is open.

  ## The `send(self(), :process)` loop

  Like the Acceptor, the Worker uses `send(self(), :process)` to turn
  synchronous I/O (`:gen_tcp.recv/3`) into a mailbox loop:

  ```
  init/1  ──► send(self(), :process) ──► returns {:ok, state}
                                               │
  BEAM delivers :process message to mailbox    │
                                               ▼
  handle_info(:process, state) ──► recv HTTP bytes ──► run handler
                   ▲                                          │
                   │                         keep-alive?      │
                   └── send(self(), :process) ◄── yes ────────┘
                                             no ──► close + {:stop, :normal, state}
  ```

  Each iteration is a separate mailbox round-trip. Between iterations, the
  BEAM scheduler can run other Workers, the Acceptor, or any other process
  on the node. This is **cooperative concurrency** at the OTP level.

  ## "Let it crash" in action

  If a user handler raises an exception, `run_handler/2` catches it and
  returns a 500 response. The Worker keeps running (no crash).

  If the HttpParser itself crashes (say, a malformed packet causes an
  unexpected pattern-match failure), the Worker process exits abnormally.
  The DynamicSupervisor reaps it. The TCP connection dies — the client
  gets a RST and can retry. The rest of the server is completely unaffected.

  This is "let it crash": individual connection failures are isolated to
  one Worker process. There is no try/catch around the entire server.

  ## Keep-alive

  HTTP/1.1 defaults to `Connection: keep-alive`. The Worker checks the
  `connection` header after each response:
  - `keep-alive`: loop back to `:process` to serve the next request.
  - `close` (or HTTP/1.0): close the socket and exit normally.

  A Worker that has served 1000 requests on the same keep-alive connection
  is just one long-lived gen_server process — still just ~4 KiB of memory.
  """

  use GenServer

  require Logger

  alias CodingAdventures.ConduitOtp.{
    HttpParser,
    Router,
    Request
  }

  @doc "Start a Worker for the given accepted socket and Application snapshot."
  @spec start_link(:gen_tcp.socket(), map) :: GenServer.on_start()
  def start_link(socket, app_snapshot) do
    GenServer.start_link(__MODULE__, {socket, app_snapshot})
  end

  # ── GenServer callbacks ───────────────────────────────────────────────────────

  @impl true
  def init({socket, app}) do
    # We DON'T call :gen_tcp.controlling_process here — the Acceptor does it
    # AFTER start_link returns. The Worker's init just schedules :process.
    send(self(), :process)
    {:ok, %{socket: socket, app: app}}
  end

  @impl true
  def handle_info(:process, %{socket: socket, app: app} = state) do
    case HttpParser.read_request(socket) do
      {:ok, {method, path, headers, body}} ->
        req = Request.from_parsed(method, path, headers, body)
        response = run_handler(req, app)
        :ok = send_response(socket, response)

        if keep_alive?(headers) do
          send(self(), :process)
          {:noreply, state}
        else
          :gen_tcp.close(socket)
          {:stop, :normal, state}
        end

      {:error, :closed} ->
        # Client closed the connection cleanly (FIN). Exit normally.
        {:stop, :normal, state}

      {:error, :timeout} ->
        # Client sent nothing for 15 s. Close and exit.
        :gen_tcp.close(socket)
        {:stop, :normal, state}

      {:error, reason} ->
        Logger.debug("HTTP parse error #{inspect(reason)}")
        # Send a 400 Bad Request and close.
        send_response(socket, {400, %{"content-type" => "text/plain"}, "Bad Request"})
        :gen_tcp.close(socket)
        {:stop, :normal, state}
    end
  end

  # Ignore unexpected messages so a stray send doesn't crash a Worker.
  def handle_info(_other, state), do: {:noreply, state}

  @impl true
  def terminate(_reason, %{socket: socket}) do
    :gen_tcp.close(socket)
    :ok
  end

  # ── Handler dispatch ─────────────────────────────────────────────────────────

  # This is the heart of the framework: run before_filters → route →
  # after_filters, catching halts and exceptions at each stage.
  #
  # Elixir requires `rescue` before `catch` in a try block. `rescue` catches
  # Elixir exceptions (raised with `raise`). `catch` catches throws and exits.
  defp run_handler(%Request{} = req, app) do
    try do
      # 1. Before filters — run in registration order.
      #    A filter returns nil to pass through, or a response tuple to halt.
      req = run_before_filters(req, app)

      # 2. Route dispatch — find a matching route and call the handler.
      {req, response} = dispatch(req, app)

      # 3. After filters — can rewrite the response.
      run_after_filters(req, app, response)
    rescue
      # `rescue` catches raised exceptions (Elixir structs implementing Exception).
      # This covers `raise "boom"`, `raise MyError, message: "..."`, etc.
      e ->
        stack = __STACKTRACE__
        Logger.error(fn -> Exception.format(:error, e, stack) end)

        case app.error_handler do
          nil ->
            {500, %{}, "Internal Server Error"}

          id ->
            err_req = %{req | env: Map.put(req.env, "conduit.error", Exception.message(e))}
            call_handler(app, id, err_req) ||
              {500, %{}, "Internal Server Error"}
        end
    catch
      # `catch` catches throw and exit. Our halt pattern uses throw.
      :throw, {:conduit_halt, status, body, headers} ->
        {status, headers, body}

      kind, reason ->
        stack = __STACKTRACE__
        Logger.error(fn -> Exception.format(kind, reason, stack) end)

        # Try the error handler if registered.
        case app.error_handler do
          nil ->
            {500, %{}, "Internal Server Error"}

          id ->
            err_req = %{req | env: Map.put(req.env, "conduit.error", inspect(reason))}
            call_handler(app, id, err_req) ||
              {500, %{}, "Internal Server Error"}
        end
    end
  end

  defp run_before_filters(req, app) do
    # Iterate before_filters; if a filter returns a response tuple, short-circuit
    # by throwing a halt so the outer try/catch in run_handler picks it up.
    Enum.reduce_while(app.before_filters, req, fn id, current_req ->
      case call_handler(app, id, current_req) do
        nil ->
          {:cont, current_req}

        {status, headers, body} when is_integer(status) and is_map(headers) and is_binary(body) ->
          # Filter returned a response — throw as halt so the outer handler
          # short-circuits immediately.
          throw({:conduit_halt, status, body, headers})

        _ ->
          {:cont, current_req}
      end
    end)
  end

  defp dispatch(req, app) do
    case Router.match(app.routes, req.method, req.path) do
      {:ok, handler_id, params} ->
        matched_req = %{req | params: params}
        response = call_handler(app, handler_id, matched_req) ||
          {200, %{}, ""}
        {matched_req, response}

      :not_found ->
        case app.not_found_handler do
          nil ->
            {req, {404, %{"content-type" => "text/plain"}, "Not Found"}}

          id ->
            response = call_handler(app, id, req) ||
              {404, %{}, "Not Found"}
            {req, response}
        end
    end
  end

  defp run_after_filters(req, app, response) do
    Enum.reduce(app.after_filters, response, fn id, current_response ->
      case call_handler(app, id, req) do
        nil -> current_response
        {_s, _h, _b} = new_response -> new_response
        _ -> current_response
      end
    end)
  end

  # Look up handler by ID and call it. Returns the result or nil if id not found.
  defp call_handler(app, id, req) do
    case Map.fetch(app.handlers, id) do
      {:ok, fun} -> fun.(req)
      :error -> nil
    end
  end

  # ── Response serialisation ────────────────────────────────────────────────────

  # Encode a `{status, headers, body}` triple as an HTTP/1.1 response and
  # send it over the TCP socket.
  defp send_response(socket, {status, headers, body}) do
    # Normalise body to binary.
    body_bin = to_string(body)

    status_line = "HTTP/1.1 #{status} #{status_text(status)}\r\n"

    # Always include Content-Length so the client knows when the body ends.
    # Merge with caller-provided headers (caller headers take precedence for
    # things like Content-Type, Location, etc.).
    content_length = byte_size(body_bin)

    default_headers = %{
      "content-length" => Integer.to_string(content_length),
      "connection" => "keep-alive"
    }

    merged = Map.merge(default_headers, headers)

    header_lines =
      merged
      |> Enum.map(fn {k, v} -> "#{k}: #{v}\r\n" end)
      |> Enum.join()

    response = status_line <> header_lines <> "\r\n" <> body_bin
    :gen_tcp.send(socket, response)
  end

  # ── Keep-alive ────────────────────────────────────────────────────────────────

  defp keep_alive?(headers) do
    connection_header =
      headers
      |> Map.get("connection", "keep-alive")
      |> String.downcase()

    connection_header != "close"
  end

  # ── HTTP status text ──────────────────────────────────────────────────────────

  defp status_text(200), do: "OK"
  defp status_text(201), do: "Created"
  defp status_text(204), do: "No Content"
  defp status_text(301), do: "Moved Permanently"
  defp status_text(302), do: "Found"
  defp status_text(400), do: "Bad Request"
  defp status_text(401), do: "Unauthorized"
  defp status_text(403), do: "Forbidden"
  defp status_text(404), do: "Not Found"
  defp status_text(405), do: "Method Not Allowed"
  defp status_text(500), do: "Internal Server Error"
  defp status_text(503), do: "Service Unavailable"
  defp status_text(n), do: Integer.to_string(n)
end
