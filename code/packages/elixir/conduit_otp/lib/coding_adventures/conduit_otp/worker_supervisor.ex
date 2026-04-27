defmodule CodingAdventures.ConduitOtp.WorkerSupervisor do
  @moduledoc """
  Teaching topic: `DynamicSupervisor` and `:temporary` restart.

  ## What is a DynamicSupervisor?

  A `Supervisor` has a *static* child list defined at startup. You declare
  "start child A, child B, child C" in `init/1`, and the supervisor starts
  exactly those. Good for services that always exist (database pool,
  route table, etc.).

  A `DynamicSupervisor` has a *dynamic* child list — you call
  `DynamicSupervisor.start_child/2` at runtime to add children. Good for
  resources that are created on demand (one per HTTP connection, one per
  user session, etc.).

  ## Restart strategies for workers

  | `:restart` value | Meaning |
  |-----------------|---------|
  | `:permanent`    | Always restarted after any exit (normal or crash). |
  | `:transient`    | Restarted only after abnormal exits (crashes). |
  | `:temporary`    | **Never restarted**, regardless of exit reason. |

  We use `:temporary` for Workers. Why? A Worker is tied to one TCP
  connection. When that connection closes (normally or due to a crash),
  there is nothing to restart — the socket is gone. Restarting would just
  create a Worker with a dead socket.

  ## Why DynamicSupervisor at all, if we never restart?

  Three reasons:

  1. **Crash accounting.** Even with `:temporary`, the supervisor tracks
     crashes. A burst of 1000 crashes/second from malformed requests is
     visible in `:observer.start/0`.

  2. **Graceful shutdown.** When the application stops, the supervisor sends
     `:shutdown` to each Worker and waits `shutdown: 5_000` ms. This gives
     in-flight requests time to finish sending their response. Bare `spawn/1`
     processes get killed with no warning.

  3. **OTP hygiene.** Every process should be in the supervision tree. The
     BEAM's `:observer` shows the full tree; orphan processes are a code smell
     and a memory leak risk.

  ## Naming

  Like `RouteTable`, we accept a `:name` in opts so multiple server instances
  can coexist on the same BEAM node.
  """

  use DynamicSupervisor

  alias CodingAdventures.ConduitOtp.Worker

  @doc "Start the DynamicSupervisor, registered under `:name` (default: `__MODULE__`)."
  @spec start_link(keyword) :: Supervisor.on_start()
  def start_link(opts) do
    name = Keyword.get(opts, :name, __MODULE__)
    DynamicSupervisor.start_link(__MODULE__, opts, name: name)
  end

  @impl true
  def init(_opts) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @doc """
  Start a `Worker` child under this supervisor for the given socket.

  `route_snapshot` is the `%Application{}` the worker will use for this
  connection. We snapshot at accept-time so that a hot_reload while the
  request is in-flight doesn't change the routing mid-request.
  """
  @spec start_worker(:gen_tcp.socket(), map, keyword) :: DynamicSupervisor.on_start_child()
  def start_worker(socket, route_snapshot, opts \\ []) do
    name = Keyword.get(opts, :supervisor_name, __MODULE__)

    spec = %{
      id: Worker,
      start: {Worker, :start_link, [socket, route_snapshot]},
      # `:temporary` — never restart a dead TCP connection.
      restart: :temporary,
      # Give a graceful-shutdown window of 5 seconds.
      shutdown: 5_000,
      type: :worker
    }

    DynamicSupervisor.start_child(name, spec)
  end
end
