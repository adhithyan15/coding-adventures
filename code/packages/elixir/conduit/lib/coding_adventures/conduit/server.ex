defmodule CodingAdventures.Conduit.Server do
  @moduledoc """
  Boots a Conduit `Application` against a TCP port.

  ## Lifecycle

      app = Application.new() |> Application.get("/", &handler/1)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
      Server.serve(server)            # blocks the calling process
      # ^ or:
      Server.serve_background(server) # returns immediately, server runs on a Rust thread
      Server.stop(server)             # signal shutdown

  ## What `start_link/2` does

  1. Starts a `Conduit.Dispatcher` GenServer holding the compiled handler map.
  2. Calls `Native.new_app/0` to allocate the Rust-side resource.
  3. Replays the Application struct into the Rust app (routes, filters,
     not_found, error_handler, settings).
  4. Calls `Native.new_server/5` to bind the TCP socket; passes the
     dispatcher's PID so the Rust I/O threads know where to send requests.
  5. Returns a `Server` struct holding the resource and dispatcher PID.

  The Server is itself a plain struct, NOT a GenServer — its lifecycle is
  tied to the dispatcher. If you want supervision, put the dispatcher (and
  its `start_link` shim) under your application's supervision tree.
  """

  alias CodingAdventures.Conduit.{Application, Dispatcher, Native}

  defstruct [:resource, :dispatcher_pid, :host, :port]

  @type t :: %__MODULE__{
          resource: reference,
          dispatcher_pid: pid,
          host: String.t(),
          port: non_neg_integer
        }

  @doc """
  Start the dispatcher and bind the server.

  Options:
  - `:host` (default `"127.0.0.1"`)
  - `:port` (default `3000`; pass `0` to let the OS pick a free port)
  - `:max_connections` (default `128`)
  """
  @spec start_link(Application.t(), keyword) :: {:ok, t} | {:error, term}
  def start_link(%Application{} = app, opts \\ []) do
    host = Keyword.get(opts, :host, "127.0.0.1")
    port = Keyword.get(opts, :port, 3000)
    max_conn = Keyword.get(opts, :max_connections, 128)

    with {:ok, dispatcher_pid} <- Dispatcher.start_link(app.handlers) do
      app_resource = build_native_app(app)

      server_resource =
        Native.new_server(app_resource, host, port, max_conn, dispatcher_pid)

      bound_port = Native.server_local_port(server_resource)

      {:ok,
       %__MODULE__{
         resource: server_resource,
         dispatcher_pid: dispatcher_pid,
         host: host,
         port: bound_port
       }}
    end
  end

  @doc "Block the calling process running the server until `stop/1`."
  @spec serve(t) :: :ok
  def serve(%__MODULE__{resource: r}), do: Native.server_serve(r)

  @doc "Run the server on a background Rust thread; returns immediately."
  @spec serve_background(t) :: :ok
  def serve_background(%__MODULE__{resource: r}),
    do: Native.server_serve_background(r)

  @doc "Signal the server to stop. Safe from any process."
  @spec stop(t) :: :ok
  def stop(%__MODULE__{resource: r, dispatcher_pid: pid}) do
    Native.server_stop(r)
    if Process.alive?(pid), do: GenServer.stop(pid, :normal)
    :ok
  end

  @doc "The bound port (useful when port=0 was requested)."
  @spec local_port(t) :: non_neg_integer
  def local_port(%__MODULE__{resource: r}), do: Native.server_local_port(r)

  @doc "Whether the server background thread is running."
  @spec running?(t) :: boolean
  def running?(%__MODULE__{resource: r}) do
    # The NIF returns the atom `:true` or `:false`; in Elixir these are
    # the boolean values `true`/`false` themselves (booleans ARE atoms).
    Native.server_running(r) == true
  end

  # ── Internal: replay an Application onto a Native resource ────────────────

  defp build_native_app(%Application{} = app) do
    res = Native.new_app()

    Enum.each(app.routes, fn %{method: m, pattern: p, handler_id: id} ->
      Native.app_add_route(res, m, p, id)
    end)

    Enum.each(app.before_filters, &Native.app_add_before(res, &1))
    Enum.each(app.after_filters, &Native.app_add_after(res, &1))

    if app.not_found_handler, do: Native.app_set_not_found(res, app.not_found_handler)
    if app.error_handler, do: Native.app_set_error_handler(res, app.error_handler)

    Enum.each(app.settings, fn {k, v} ->
      Native.app_set_setting(res, to_string(k), to_string(v))
    end)

    res
  end
end
