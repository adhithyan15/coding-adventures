defmodule CodingAdventures.ConduitOtp.RouteTable do
  @moduledoc """
  Teaching topic: `Agent` — the simplest OTP stateful process.

  ## What is an `Agent`?

  `Agent` is the simplest OTP process behaviour: a process holding exactly
  one value (its "state"). You can `get` the value and `update` it. That's
  it. Internally it is just a gen_server with a single `handle_call` pattern.

  Use an `Agent` when you need shared mutable state but don't need custom
  messages or complex logic. For more complex state machines, use `GenServer`.

  ## What the RouteTable does

  It holds the compiled `%Application{}` struct — the full set of routes,
  filters, and handlers for this server instance. Workers call `snapshot/0`
  at startup to get a copy (it's an immutable map — safe to snapshot).

  ## Hot reload

  `hot_reload/1` atomically replaces the Application struct. The next Worker
  to `snapshot/0` gets the new routes. In-flight Workers keep their snapshot
  (Elixir structs are immutable values, not pointers).

  This is the "hot code swap" pattern at the application level — no need to
  restart the server, no need for a rolling deploy. Just call `hot_reload/1`.

  ## Naming: why a module-level `name:`?

  We register the Agent under its own module name (`name: __MODULE__`). This
  means any process in the node can call `RouteTable.snapshot()` without
  needing to pass the PID around. The OTP `name` registry maps module names
  to PIDs in the local node's process table.

  There is a subtle problem: if you start two server instances on the same
  node, they'd share the same RouteTable name. For our purposes (one server
  per node) this is fine. A production-grade version would namespace by
  `{__MODULE__, server_id}` and use the `Registry` behaviour.
  """

  use Agent

  alias CodingAdventures.ConduitOtp.Application

  @doc """
  Start the RouteTable agent with the given Application struct.

  `opts` must contain `:app` (the `%Application{}`) and optionally `:name`
  for multi-server setups. Defaults to registering as `__MODULE__`.
  """
  @spec start_link(keyword) :: Agent.on_start()
  def start_link(opts) do
    app = Keyword.fetch!(opts, :app)
    name = Keyword.get(opts, :name, __MODULE__)
    Agent.start_link(fn -> app end, name: name)
  end

  @doc """
  Return the current `%Application{}` snapshot.

  This is a synchronous `Agent.get` call — it sends a message to the Agent
  process and waits for the reply. The reply is the full struct, which is an
  immutable value. The caller owns a safe snapshot.
  """
  @spec snapshot(GenServer.server()) :: Application.t()
  def snapshot(name \\ __MODULE__) do
    Agent.get(name, & &1)
  end

  @doc """
  Replace the Application struct atomically.

  The next `snapshot/0` call will return `new_app`. In-flight Workers
  are unaffected — they hold their own snapshot copy.

  This is atomic at the Erlang message level: the update message is processed
  serially by the Agent's gen_server loop; no request can see a half-updated
  struct.
  """
  @spec hot_reload(Application.t(), GenServer.server()) :: :ok
  def hot_reload(new_app, name \\ __MODULE__) do
    Agent.update(name, fn _old -> new_app end)
  end
end
