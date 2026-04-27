defmodule CodingAdventures.ConduitOtp.OtpSupervisor do
  @moduledoc """
  Teaching topic: Supervisor strategies and restart budgets.

  ## What is a Supervisor?

  A Supervisor is an OTP behaviour whose entire job is:
  1. **Start** a list of child processes according to a spec.
  2. **Monitor** those children (via links).
  3. **Restart** them when they die according to a strategy.

  You almost never write a `handle_call` in a Supervisor — it has no logic,
  only policy. This is the "policy and mechanism separation" principle.

  ## Supervisor strategies

  | Strategy       | Meaning                                               | Our choice? |
  |----------------|-------------------------------------------------------|-------------|
  | `:one_for_one` | Restart only the dead child; siblings unaffected      | ✓ Yes       |
  | `:one_for_all` | If any child dies, restart ALL children               |             |
  | `:rest_for_one`| Restart the dead child AND all children defined after |             |

  We choose `:one_for_one` because our three children are independent:
  - `RouteTable` (Agent) — holds routes; unrelated to I/O.
  - `WorkerSupervisor` (DynamicSupervisor) — manages per-request workers.
  - `Acceptor` (gen_server) — owns the listen socket.

  If the `Acceptor` crashes (listen socket closed), restarting only it makes
  sense — the route table and existing workers are fine.

  ## Restart budget (intensity / period)

  `max_restarts: 5, max_seconds: 10` means: "if more than 5 restarts happen
  within any 10-second window, the supervisor itself gives up and exits."

  This stops infinite-crash loops. If a child crashes immediately on every
  restart (e.g. the port is permanently blocked), we don't want to spin
  forever — we want to escalate. The supervisor exits, and its parent
  (the OTP Application) may try to restart it, or the whole application
  shuts down and the node's crash reporter logs a clear error.

  ## Per-server isolation

  Each call to `Server.start_link/2` starts a *new* `OtpSupervisor`
  instance (with a unique name derived from the port). Multiple server
  instances on the same node do not interfere.

  ## Supervision tree shape

  ```
  OtpSupervisor  (this module, :one_for_one)
  ├─ RouteTable         (Agent)
  ├─ WorkerSupervisor   (DynamicSupervisor)
  └─ Acceptor           (GenServer)
  ```

  The ordering matters: `RouteTable` is started first so `Acceptor` and
  Workers can immediately call `RouteTable.snapshot/0` during their `init/1`.
  `WorkerSupervisor` is second so `Acceptor` can call
  `WorkerSupervisor.start_worker/2` as soon as it accepts a connection.
  """

  use Supervisor

  alias CodingAdventures.ConduitOtp.{RouteTable, WorkerSupervisor, Acceptor}

  @doc """
  Start a supervision tree for one server instance.

  `opts` must contain:
  - `:app` — the `%Application{}` struct.
  - `:port` — TCP port to bind (0 = OS-assigned).
  - `:host` — bind address string (default `"127.0.0.1"`).

  The supervisor is registered under a name derived from the port so
  multiple servers can coexist on the same node.
  """
  @spec start_link(keyword) :: Supervisor.on_start()
  def start_link(opts) do
    # Name the supervisor so `Server.stop/1` can find it by name.
    name = Keyword.get(opts, :supervisor_name, __MODULE__)
    Supervisor.start_link(__MODULE__, opts, name: name)
  end

  @impl true
  def init(opts) do
    # Use the same opts name-scoping for all children so they don't collide
    # when multiple OtpSupervisors exist on the same BEAM node.
    rt_name = Keyword.get(opts, :route_table_name, RouteTable)
    ws_name = Keyword.get(opts, :worker_sup_name, WorkerSupervisor)

    children = [
      # 1. RouteTable first: both Acceptor and Workers snapshot from it.
      {RouteTable, Keyword.put(opts, :name, rt_name)},

      # 2. WorkerSupervisor second: Acceptor calls it for new workers.
      {WorkerSupervisor, Keyword.put(opts, :name, ws_name)},

      # 3. Acceptor last: it binds the port and starts accepting.
      {Acceptor, Keyword.merge(opts, [route_table_name: rt_name, worker_sup_name: ws_name])}
    ]

    Supervisor.init(children,
      strategy: :one_for_one,
      max_restarts: 5,
      max_seconds: 10
    )
  end
end
