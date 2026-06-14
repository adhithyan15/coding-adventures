defmodule CodingAdventures.ConduitOtp.Server do
  @moduledoc """
  Teaching topic: The public façade — tying the supervision tree together.

  ## What this is

  `Server` is the single entry point for application code. It hides all
  OTP plumbing behind three simple functions:

  - `start_link/2` — start the supervision tree, bind to a port.
  - `serve/1` — block the calling process (run a server in a script).
  - `stop/1` — shut everything down cleanly.

  ## API parity with WEB06

  The public API is identical to WEB06 (`CodingAdventures.Conduit.Server`).
  To switch between NIF and OTP implementations, change one alias:

      alias CodingAdventures.Conduit.Server   # WEB06 (Rust NIF)
      alias CodingAdventures.ConduitOtp.Server # WEB07 (pure OTP)

  The DSL (`Application.new() |> ...`), request struct, and all helper
  functions (`html/1`, `json/2`, `halt/2`, `redirect/1`) are identical.

  ## Example

      alias CodingAdventures.ConduitOtp.{Application, Server}
      import CodingAdventures.ConduitOtp.HandlerContext

      app =
        Application.new()
        |> Application.get("/", fn _ -> html("<h1>Hello!</h1>") end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
      Server.serve(server)        # blocks until Ctrl-C

  ## Under the hood

  `start_link/2` starts an `OtpSupervisor` that contains:
  - `RouteTable` (Agent) — holds the Application struct.
  - `WorkerSupervisor` (DynamicSupervisor) — manages per-connection workers.
  - `Acceptor` (GenServer) — owns the listen socket, accepts connections.

  `serve/1` simply blocks the calling process in a `receive` loop — the
  server runs on its own processes.  For scripts, this keeps the process
  alive until you `stop/1` or send a shutdown signal.
  """

  alias CodingAdventures.ConduitOtp.{Application, OtpSupervisor, Acceptor}

  defstruct [:supervisor_pid, :acceptor_name, :host, :port]

  @type t :: %__MODULE__{
          supervisor_pid: pid,
          acceptor_name: atom | pid,
          host: String.t(),
          port: non_neg_integer
        }

  @doc """
  Start the Conduit OTP server for the given `%Application{}`.

  Options:
  - `:host` — bind address (default `"127.0.0.1"`)
  - `:port` — TCP port (default `3000`; use `0` for OS-assigned)

  Returns `{:ok, %Server{}}` or `{:error, reason}`.

  The server is linked to the calling process — if the caller dies, the
  supervisor tree dies with it. Wrap in your own supervisor if you need
  it to outlive the caller.
  """
  @spec start_link(Application.t(), keyword) :: {:ok, t} | {:error, term}
  def start_link(%Application{} = app, opts \\ []) do
    host = Keyword.get(opts, :host, "127.0.0.1")
    port = Keyword.get(opts, :port, 3000)

    # Give each server instance unique process names so multiple servers
    # can coexist on the same BEAM node (important for tests).
    instance_id = :erlang.unique_integer([:positive])
    supervisor_name = :"CodingAdventures.ConduitOtp.OtpSupervisor_#{instance_id}"
    acceptor_name = :"CodingAdventures.ConduitOtp.Acceptor_#{instance_id}"
    rt_name = :"CodingAdventures.ConduitOtp.RouteTable_#{instance_id}"
    ws_name = :"CodingAdventures.ConduitOtp.WorkerSupervisor_#{instance_id}"

    sup_opts = [
      app: app,
      host: host,
      port: port,
      supervisor_name: supervisor_name,
      acceptor_name: acceptor_name,
      route_table_name: rt_name,
      worker_sup_name: ws_name
    ]

    case OtpSupervisor.start_link(sup_opts) do
      {:ok, sup_pid} ->
        # Give the Acceptor a moment to bind (init/1 is synchronous, so
        # this is instantaneous, but the OS port assignment may need a tick).
        bound_port = get_bound_port(acceptor_name)

        {:ok,
         %__MODULE__{
           supervisor_pid: sup_pid,
           acceptor_name: acceptor_name,
           host: host,
           port: bound_port
         }}

      {:error, _} = err ->
        err
    end
  end

  @doc """
  Block the calling process until the server is stopped.

  Useful in scripts and `iex` sessions. The server continues running on its
  own processes; this call just keeps the calling process from exiting.

  Press Ctrl-C or call `Server.stop/1` from another process to exit.
  """
  @spec serve(t) :: :ok
  def serve(%__MODULE__{}) do
    receive do
      _ -> :ok
    end
  end

  @doc "Stop the server and its supervision tree cleanly. Safe to call more than once."
  @spec stop(t) :: :ok
  def stop(%__MODULE__{supervisor_pid: pid}) do
    if Process.alive?(pid) do
      try do
        Supervisor.stop(pid, :normal)
      catch
        :exit, _ -> :ok
      end
    end

    :ok
  end

  @doc "Return the port the server is actually bound to. Useful when port: 0 was requested."
  @spec local_port(t) :: non_neg_integer
  def local_port(%__MODULE__{port: port}), do: port

  @doc "Return true if the supervisor process is still alive."
  @spec running?(t) :: boolean
  def running?(%__MODULE__{supervisor_pid: pid}), do: Process.alive?(pid)

  # ── Private helpers ───────────────────────────────────────────────────────────

  # Ask the Acceptor for the actual bound port. The Acceptor's init/1 has
  # already completed and bound the socket by the time start_link returns,
  # so this is safe to call immediately after OtpSupervisor.start_link.
  defp get_bound_port(acceptor_name) do
    try do
      Acceptor.local_port(acceptor_name)
    catch
      _, _ -> 0
    end
  end
end
