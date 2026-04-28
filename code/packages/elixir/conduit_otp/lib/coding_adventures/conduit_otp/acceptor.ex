defmodule CodingAdventures.ConduitOtp.Acceptor do
  @moduledoc """
  Teaching topic: `GenServer`, passive sockets, and the `send(self(), ...)` loop.

  ## What is a GenServer?

  `GenServer` (Generic Server) is the most-used OTP behaviour. It is a
  process that:
  - Holds state (returned from `init/1`).
  - Handles synchronous `call` messages — the caller blocks for a reply.
  - Handles asynchronous `cast` messages — fire-and-forget.
  - Handles arbitrary `info` messages (OS signals, timer callbacks, etc.)
    via `handle_info/2`.

  The Acceptor only uses `handle_info/2` — its sole job is the accept loop.

  ## Passive vs active TCP sockets

  `:gen_tcp` sockets have two reading modes:

  | Mode | How data arrives |
  |------|-----------------|
  | `{:active, true}` | Socket delivers packets as Erlang messages to the owner process automatically. Simple but you can't apply backpressure. |
  | `{:active, false}` | **Passive mode** — data sits in the kernel buffer. You call `recv/2,3` explicitly. You control the pace. |
  | `{:active, :once}` | Deliver ONE packet as a message, then switch back to passive. Useful for hybrid patterns. |

  We use `{:active, false}` (passive) so the Acceptor controls exactly when
  it blocks on `:gen_tcp.accept/2`. This is the "pull" model.

  ## The `send(self(), :accept)` loop

  Why not just call `:gen_tcp.accept/2` directly in `init/1`?

  Because `init/1` is synchronous — the supervisor that called `start_link/1`
  blocks until `init/1` returns. If we block forever in `init/1`, the
  supervisor deadlocks.

  The pattern: `init/1` returns immediately, and sends `self()` the
  `:accept` message. This gets delivered to `handle_info/2` *after* `init/1`
  returns. Then `handle_info` can call `:gen_tcp.accept/2` (which blocks until
  a connection arrives) and immediately send `self()` the next `:accept` after
  handing off the connection.

  This turns blocking I/O into a single-entry mailbox loop. Each iteration is
  one BEAM scheduler turn — other processes get fair time between accepts.

  ## Socket ownership and `controlling_process/2`

  When `:gen_tcp.accept/2` returns a connected socket, the Acceptor process
  *owns* that socket. "Ownership" means only one process can call `recv` on
  it. We immediately call `:gen_tcp.controlling_process(socket, worker_pid)`
  to hand ownership to the Worker that will serve the request.

  If we skip this step, the Worker's `recv` calls fail with `{:error, :einval}`.
  """

  use GenServer

  require Logger

  alias CodingAdventures.ConduitOtp.{RouteTable, WorkerSupervisor}

  @doc "Start the Acceptor, binding to the port specified in `opts`."
  @spec start_link(keyword) :: GenServer.on_start()
  def start_link(opts) do
    name = Keyword.get(opts, :acceptor_name, __MODULE__)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc "Return the port number that the listen socket was actually bound to."
  @spec local_port() :: non_neg_integer | {:error, term}
  def local_port do
    GenServer.call(__MODULE__, :local_port)
  end

  @doc "Return the port by calling GenServer.call on the named acceptor."
  @spec local_port(GenServer.server()) :: non_neg_integer | {:error, term}
  def local_port(server) do
    GenServer.call(server, :local_port)
  end

  # ── GenServer callbacks ───────────────────────────────────────────────────────

  @impl true
  def init(opts) do
    # Extract configuration from opts, falling back to sensible defaults.
    raw_host = Keyword.get(opts, :host, "127.0.0.1")
    port = Keyword.get(opts, :port, 3000)
    rt_name = Keyword.get(opts, :route_table_name, RouteTable)
    ws_name = Keyword.get(opts, :worker_sup_name, WorkerSupervisor)

    # Parse the host string into an `{a, b, c, d}` IP tuple.
    # `:gen_tcp.listen/2` accepts `{:ip, tuple}` or `{:ip, :any}`.
    ip = parse_ip(raw_host)

    # Open the listen socket with HTTP/1.1-aware packet framing.
    #
    # Key options:
    # - `:binary`          — deliver data as Elixir binaries, not char lists.
    # - `{:packet, :http_bin}` — use BEAM's built-in HTTP/1.1 framing.
    # - `{:active, false}` — passive mode: we call recv/2 explicitly.
    # - `{:reuseaddr, true}` — allow re-binding to a port in TIME_WAIT state.
    #   Without this, restarting the server quickly fails with {:error, :eaddrinuse}.
    listen_opts =
      [:binary, {:packet, :http_bin}, {:active, false}, {:reuseaddr, true}] ++
        if ip == :any, do: [], else: [{:ip, ip}]

    case :gen_tcp.listen(port, listen_opts) do
      {:ok, lsock} ->
        # Non-blocking: send `:accept` to ourselves AFTER init returns.
        send(self(), :accept)
        {:ok, %{lsock: lsock, rt_name: rt_name, ws_name: ws_name}}

      {:error, reason} ->
        # init/1 returning {:stop, reason} tells the supervisor "I couldn't
        # start". The supervisor respects the restart budget and decides
        # whether to retry.
        {:stop, {:listen_failed, reason}}
    end
  end

  # Synchronous call: return the port we're actually listening on.
  # `:inet.port/1` does a kernel call to find the bound port.
  # Useful when port=0 was requested (OS picks an ephemeral port).
  @impl true
  def handle_call(:local_port, _from, %{lsock: lsock} = state) do
    case :inet.port(lsock) do
      {:ok, port} -> {:reply, port, state}
      {:error, _} = err -> {:reply, err, state}
    end
  end

  # The accept loop: called via `send(self(), :accept)` on every iteration.
  #
  # We use a short timeout (200 ms) on :gen_tcp.accept/2 so the gen_server
  # remains responsive to synchronous `call` messages (like :local_port)
  # between accept attempts. Without a timeout, handle_info would block
  # indefinitely and the gen_server's mailbox would queue up — any `call`
  # would timeout waiting for an answer.
  @impl true
  def handle_info(:accept, %{lsock: lsock, rt_name: rt_name, ws_name: ws_name} = state) do
    case :gen_tcp.accept(lsock, 200) do
      {:ok, sock} ->
        # Snapshot the current Application struct for this connection.
        # The Worker uses this snapshot for the lifetime of the connection.
        app_snapshot = RouteTable.snapshot(rt_name)

        # Start a Worker under the DynamicSupervisor.
        case WorkerSupervisor.start_worker(sock, app_snapshot, supervisor_name: ws_name) do
          {:ok, worker_pid} ->
            # Transfer socket ownership so the Worker can call recv.
            # Must happen BEFORE the worker calls recv; we do it here
            # because we are the current owner.
            :gen_tcp.controlling_process(sock, worker_pid)

          {:error, reason} ->
            Logger.warning("WorkerSupervisor.start_worker failed: #{inspect(reason)}")
            :gen_tcp.close(sock)
        end

        # Re-arm: loop unconditionally.
        send(self(), :accept)
        {:noreply, state}

      {:error, :timeout} ->
        # No connection in 200 ms — re-arm and loop. This is NOT an error;
        # it is just the poll interval that keeps the gen_server responsive.
        send(self(), :accept)
        {:noreply, state}

      {:error, :closed} ->
        # The listen socket was closed (server shutting down). Exit normally.
        {:stop, :normal, state}

      {:error, reason} ->
        Logger.error("Accept failed: #{inspect(reason)}")
        # Re-arm even on transient errors (e.g. :econnaborted).
        send(self(), :accept)
        {:noreply, state}
    end
  end

  # ── Cleanup ──────────────────────────────────────────────────────────────────

  @impl true
  def terminate(_reason, %{lsock: lsock}) do
    # Close the listen socket when the gen_server shuts down.
    # `terminate/2` runs when:
    # - The supervisor sends :shutdown signal.
    # - The process calls GenServer.stop/1.
    # NOTE: terminate/2 does NOT run on abnormal crashes unless you call
    # `Process.flag(:trap_exit, true)` in init/1. We don't — if we crash,
    # the supervisor restarts us; the socket's OS file descriptor gets cleaned
    # up by the BEAM's port table on the old process exit.
    :gen_tcp.close(lsock)
    :ok
  end

  # ── Private helpers ───────────────────────────────────────────────────────────

  defp parse_ip("0.0.0.0"), do: :any
  defp parse_ip(host) when is_binary(host) do
    case :inet.parse_address(String.to_charlist(host)) do
      {:ok, ip_tuple} -> ip_tuple
      _ -> {127, 0, 0, 1}
    end
  end

  defp parse_ip(:any), do: :any
  defp parse_ip(tuple) when is_tuple(tuple), do: tuple
end
