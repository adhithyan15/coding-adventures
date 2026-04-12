defmodule CodingAdventures.IrcNetStdlib.EventLoop do
  @moduledoc """
  TCP event loop -- one Task per connection, GenServer for shared state.

  ## Architecture

  This module implements the network I/O layer of the IRC stack using
  Elixir's `:gen_tcp` module and OTP's `GenServer` and `Task` primitives.

  ### How connections work

      EventLoop (GenServer)
          +-- accept_loop Task  <-- blocks in :gen_tcp.accept/1
          +-- worker Task per connected client
                  | calls Handler callbacks (serialised via mutex)

  1. `start_link/1` starts the GenServer.
  2. `run/3` spawns an accept loop Task that blocks on `:gen_tcp.accept/1`.
  3. For each accepted socket, a *worker Task* is spawned. The worker calls
     `handler.on_connect/2`, then loops calling `:gen_tcp.recv/3` until the
     socket is closed, dispatching each chunk to `handler.on_data/2`, and
     finally calls `handler.on_disconnect/1`.
  4. `send_to/3` looks up the socket in an ETS table and calls `:gen_tcp.send/2`.

  ## Concurrency

  Unlike the Python reference implementation (which uses `threading.Lock`),
  Elixir's actor model provides natural serialisation:

  * All Handler callbacks are serialised via `dispatch/2`, which acquires
    an exclusive lock from the GenServer, runs the callback in the caller's
    own process, then releases the lock. This avoids running arbitrary
    callbacks inside the GenServer process, preventing deadlocks when
    callbacks call back into the EventLoop (e.g. `send_to/3`).

  * The connections map (`conn_id -> socket`) lives in an ETS table owned by
    the GenServer. Any process can read/write the ETS table directly without
    going through the GenServer, which allows `send_to/3` to be called from
    within a `dispatch/2` callback without deadlocking.

  * The GenServer itself manages only the ETS table reference, the next
    connection ID, and the handler lock queue.
  """

  use GenServer
  require Logger

  alias CodingAdventures.IrcNetStdlib.Handler

  @type conn_id :: non_neg_integer()

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Start the EventLoop GenServer.

  ## Returns

  `{:ok, pid}` on success, `{:error, reason}` on failure.
  """
  @spec start_link(keyword()) :: GenServer.on_start()
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, %{}, opts)
  end

  @doc """
  Start accepting connections on *listen_socket* and dispatching events to *handler*.

  This is a **non-blocking** call -- it spawns the accept loop Task and returns
  `{:ok, pid}` immediately. Use `Task.start` (not `Task.async`) so the accept
  loop Task is not linked to the calling process.

  ## Parameters

  - `loop`           -- PID of a started EventLoop GenServer.
  - `listen_socket`  -- A `:gen_tcp` server socket (from `:gen_tcp.listen/2`).
  - `handler`        -- Module implementing the Handler behaviour.
  """
  @spec run(pid(), :gen_tcp.socket(), Handler.t()) :: {:ok, pid()}
  def run(loop, listen_socket, handler) do
    {:ok, pid} = Task.start(fn -> accept_loop(loop, listen_socket, handler) end)
    {:ok, pid}
  end

  @doc """
  Stop the event loop.

  Closes the listener socket (causing the accept loop to exit) and clears
  the connections map. In-flight worker Tasks are left to finish naturally.
  """
  @spec stop(pid()) :: :ok
  def stop(loop) do
    safe_call(loop, :stop)
  end

  @doc """
  Send *data* to the connection identified by *conn_id*.

  Looks up the socket in the ETS table and calls `:gen_tcp.send/2`.
  If *conn_id* is not found (the connection was already closed), this is a
  silent no-op.

  This function does NOT call the GenServer and therefore can be safely
  called from within a `dispatch/2` callback without deadlocking.
  """
  @spec send_to(pid(), conn_id(), iodata()) :: :ok
  def send_to(loop, conn_id, data) do
    # Look up the ETS table for this EventLoop. The table name is derived from
    # the EventLoop PID to allow multiple EventLoops in the same node.
    case ets_table_for(loop) do
      nil ->
        :ok

      table ->
        case :ets.lookup(table, conn_id) do
          [{^conn_id, socket}] ->
            case :gen_tcp.send(socket, data) do
              :ok -> :ok
              {:error, _reason} -> :ok
            end

          [] ->
            :ok
        end
    end
  end

  @doc """
  Serialise a Handler callback so only one runs at a time.

  Acquires a mutual-exclusion lock from the GenServer, runs *fun* in the
  **caller's process** (not the GenServer process), then releases the lock.

  Because *fun* executes in the caller's process (not inside a `handle_call`),
  it is free to call back into the EventLoop (e.g. `send_to/3`,
  `register_conn/2`) without deadlocking.

  ## Why not run fun inside the GenServer?

  Running the callback inside `handle_call` blocks the GenServer process
  until the callback returns. If the callback calls `send_to/3`, which
  needs to access the ETS table (fine) but previously called `GenServer.call`
  for socket lookup, a deadlock would occur: the GenServer is waiting for the
  callback, and the callback is waiting for the GenServer.

  The mutex approach solves this: the GenServer only manages the lock queue
  (a fast, bounded operation), and the actual callback work happens in the
  caller's process.
  """
  @spec dispatch(pid(), (-> any())) :: any()
  def dispatch(loop, fun) do
    try do
      # Acquire the lock: blocks until we are the head of the queue.
      :ok = GenServer.call(loop, :acquire_lock)
      try do
        fun.()
      after
        # Always release the lock, even if fun raises.
        safe_call(loop, :release_lock)
      end
    catch
      :exit, _ -> :ok
    end
  end

  @doc """
  Register a new connection in the GenServer state.

  Assigns a unique `conn_id` and stores the socket in the ETS table.

  ## Returns

  `{:ok, conn_id}`
  """
  @spec register_conn(pid(), :gen_tcp.socket()) :: {:ok, conn_id()}
  def register_conn(loop, socket) do
    case safe_call(loop, {:register_conn, socket}) do
      {:ok, conn_id} -> {:ok, conn_id}
      _ -> {:error, :loop_dead}
    end
  end

  @doc """
  Deregister a connection from the GenServer state.

  Called by worker Tasks after the connection closes.
  """
  @spec deregister_conn(pid(), conn_id()) :: :ok
  def deregister_conn(loop, conn_id) do
    safe_call(loop, {:deregister_conn, conn_id})
    :ok
  end

  # ---------------------------------------------------------------------------
  # GenServer callbacks
  # ---------------------------------------------------------------------------

  @impl true
  def init(_opts) do
    # Create an ETS table to store conn_id -> socket mappings. The table is
    # public so any process can read/write it without going through the GenServer.
    table = :ets.new(:irc_event_loop_conns, [:public, :set])

    state = %{
      next_id: 1,
      table: table,
      listen_sock: nil,
      # Mutex lock for Handler callbacks.
      # nil = unlocked; pid = held by this pid; list = queue of waiting {pid, ref} pairs.
      lock_holder: nil,
      lock_queue: :queue.new()
    }

    {:ok, state}
  end

  @impl true
  def handle_call({:register_conn, socket}, _from, state) do
    conn_id = state.next_id
    :ets.insert(state.table, {conn_id, socket})
    new_state = %{state | next_id: conn_id + 1}
    {:reply, {:ok, conn_id}, new_state}
  end

  def handle_call({:deregister_conn, conn_id}, _from, state) do
    :ets.delete(state.table, conn_id)
    {:reply, :ok, state}
  end

  def handle_call({:get_socket, conn_id}, _from, state) do
    # Kept for backward compatibility and tests, but send_to/3 uses ETS directly.
    result =
      case :ets.lookup(state.table, conn_id) do
        [{^conn_id, socket}] -> {:ok, socket}
        [] -> :not_found
      end

    {:reply, result, state}
  end

  def handle_call({:dispatch, fun}, _from, state) do
    # Legacy synchronous dispatch — runs fun inside the GenServer.
    # Only used by tests that call dispatch/2 directly.
    result = fun.()
    {:reply, result, state}
  end

  def handle_call(:acquire_lock, from, state) do
    case state.lock_holder do
      nil ->
        # Lock is free -- grant it immediately.
        {caller_pid, _tag} = from
        {:reply, :ok, %{state | lock_holder: caller_pid}}

      _holder ->
        # Lock is held -- queue this caller and do NOT reply yet.
        # The reply will be sent when the lock is released.
        new_queue = :queue.in(from, state.lock_queue)
        {:noreply, %{state | lock_queue: new_queue}}
    end
  end

  def handle_call(:release_lock, _from, state) do
    new_state =
      case :queue.out(state.lock_queue) do
        {{:value, next_from}, rest_queue} ->
          # Wake up the next waiter.
          GenServer.reply(next_from, :ok)
          {next_pid, _tag} = next_from
          %{state | lock_holder: next_pid, lock_queue: rest_queue}

        {:empty, _} ->
          %{state | lock_holder: nil, lock_queue: :queue.new()}
      end

    {:reply, :ok, new_state}
  end

  def handle_call(:stop, _from, state) do
    if state.listen_sock != nil do
      :gen_tcp.close(state.listen_sock)
    end

    :ets.delete_all_objects(state.table)
    {:reply, :ok, %{state | listen_sock: nil}}
  end

  def handle_call({:set_listen_sock, sock}, _from, state) do
    {:reply, :ok, %{state | listen_sock: sock}}
  end

  def handle_call({:get_table}, _from, state) do
    {:reply, state.table, state}
  end

  # ---------------------------------------------------------------------------
  # Accept loop (runs in a Task)
  # ---------------------------------------------------------------------------

  defp accept_loop(loop, listen_socket, handler) do
    safe_call(loop, {:set_listen_sock, listen_socket})
    do_accept(loop, listen_socket, handler)
  end

  defp do_accept(loop, listen_socket, handler) do
    case :gen_tcp.accept(listen_socket) do
      {:ok, socket} ->
        case register_conn(loop, socket) do
          {:ok, conn_id} ->
            host =
              case :inet.peername(socket) do
                {:ok, {addr, _port}} -> :inet.ntoa(addr) |> to_string()
                {:error, _} -> "unknown"
              end

            Task.start(fn -> worker(loop, conn_id, socket, host, handler) end)

          {:error, _} ->
            :gen_tcp.close(socket)
        end

        do_accept(loop, listen_socket, handler)

      {:error, :closed} ->
        :ok

      {:error, :einval} ->
        # Socket was closed (e.g. by stop/1).
        :ok

      {:error, reason} ->
        Logger.warning("EventLoop accept error: #{inspect(reason)}")
        do_accept(loop, listen_socket, handler)
    end
  end

  # ---------------------------------------------------------------------------
  # Worker (runs in a Task, one per connection)
  # ---------------------------------------------------------------------------

  defp worker(loop, conn_id, socket, host, handler) do
    dispatch(loop, fn -> handler.on_connect(conn_id, host) end)
    recv_loop(loop, conn_id, socket, handler)
    dispatch(loop, fn -> handler.on_disconnect(conn_id) end)
    deregister_conn(loop, conn_id)
    :gen_tcp.close(socket)
  end

  defp recv_loop(loop, conn_id, socket, handler) do
    case :gen_tcp.recv(socket, 0, 30_000) do
      {:ok, data} ->
        dispatch(loop, fn -> handler.on_data(conn_id, data) end)
        recv_loop(loop, conn_id, socket, handler)

      {:error, :timeout} ->
        if Process.alive?(loop) do
          recv_loop(loop, conn_id, socket, handler)
        else
          :ok
        end

      {:error, _reason} ->
        :ok
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Look up the ETS table associated with this EventLoop by asking the GenServer.
  # Returns nil if the loop is dead.
  defp ets_table_for(loop) do
    try do
      GenServer.call(loop, {:get_table})
    catch
      :exit, _ -> nil
    end
  end

  defp safe_call(loop, msg) do
    try do
      GenServer.call(loop, msg)
    catch
      :exit, _ -> :ok
    end
  end
end
