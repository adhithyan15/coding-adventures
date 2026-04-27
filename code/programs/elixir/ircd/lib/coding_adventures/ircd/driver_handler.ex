defmodule CodingAdventures.Ircd.DriverHandler do
  @moduledoc """
  Bridges `irc_net_stdlib` and `irc_server`.

  The `irc_net_stdlib` event loop calls three lifecycle callbacks on a module
  implementing `CodingAdventures.IrcNetStdlib.Handler`:

  * `on_connect(conn_id, host)` -- a new TCP connection arrived.
  * `on_data(conn_id, data)`    -- raw bytes from an established connection.
  * `on_disconnect(conn_id)`    -- the TCP connection has closed.

  `DriverHandler` translates these raw-bytes events into structured `Message`
  objects that `IrcServer` can process, and sends the resulting
  `{conn_id, Message}` responses back over the wire via `EventLoop.send_to/3`.

  ## Wiring diagram

      TCP socket
         | raw bytes
      EventLoop.on_data()           <- irc_net_stdlib
         | conn_id, raw bytes
      DriverHandler.on_data()       <- THIS MODULE
         | feeds bytes into per-connection Framer
      Framer.frames()               <- irc_framing
         | "NICK alice"
      IrcProto.parse()              <- irc_proto
         | Message(command: "NICK")
      IrcServer.on_message()        <- irc_server
         | [{conn_id, Message}]
      IrcProto.serialize()
         | ":irc.local 001 alice :Welcome\\r\\n"
      EventLoop.send_to()           <- irc_net_stdlib
         | bytes on the wire

  ## State

  `DriverHandler` is a GenServer. Its state contains:

  - `:server_state` -- the `IrcServer` pure state map.
  - `:framers`      -- map from `conn_id` to `Framer` struct.
  - `:loop`         -- PID of the `EventLoop` GenServer.

  ## Why a GenServer?

  The `irc_net_stdlib` event loop calls all Handler callbacks from within
  `EventLoop.dispatch/2`, which already serialises them via the EventLoop
  GenServer. However, `DriverHandler` itself is a GenServer to hold mutable
  state (the server state and framers map) safely across callback invocations.

  The `EventLoop` dispatches callbacks to the `DriverHandler` GenServer,
  providing double serialisation — but since `EventLoop.dispatch` is
  synchronous (a `GenServer.call`), the DriverHandler callbacks execute
  one at a time regardless.
  """

  use GenServer

  alias CodingAdventures.IrcFraming.Framer
  alias CodingAdventures.IrcServer
  alias CodingAdventures.IrcProto
  alias CodingAdventures.IrcNetStdlib.EventLoop

  @behaviour CodingAdventures.IrcNetStdlib.Handler

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Start the DriverHandler GenServer.

  ## Parameters

  - `server_state` -- initial `IrcServer` state (from `IrcServer.new/1`).
  - `loop`         -- PID of the `EventLoop` GenServer.
  - `opts`         -- GenServer options (e.g. `name: :handler`).
  """
  @spec start_link(map(), pid(), keyword()) :: GenServer.on_start()
  def start_link(server_state, loop, opts \\ []) do
    GenServer.start_link(__MODULE__, {server_state, loop}, opts)
  end

  # ---------------------------------------------------------------------------
  # Handler behaviour callbacks (called by the EventLoop's dispatch mechanism)
  # ---------------------------------------------------------------------------

  @impl CodingAdventures.IrcNetStdlib.Handler
  def on_connect(conn_id, host) do
    pid = self_or_named()
    GenServer.call(pid, {:on_connect, conn_id, host})
  end

  @impl CodingAdventures.IrcNetStdlib.Handler
  def on_data(conn_id, data) do
    pid = self_or_named()
    GenServer.call(pid, {:on_data, conn_id, data})
  end

  @impl CodingAdventures.IrcNetStdlib.Handler
  def on_disconnect(conn_id) do
    pid = self_or_named()
    GenServer.call(pid, {:on_disconnect, conn_id})
  end

  # The DriverHandler is registered under this name so callbacks can find it.
  defp self_or_named do
    case Process.whereis(__MODULE__) do
      nil -> raise "DriverHandler not started or not registered as #{__MODULE__}"
      pid -> pid
    end
  end

  # ---------------------------------------------------------------------------
  # GenServer callbacks
  # ---------------------------------------------------------------------------

  @impl true
  def init({server_state, loop}) do
    state = %{
      server_state: server_state,
      framers: %{},
      loop: loop
    }

    {:ok, state}
  end

  @impl true
  def handle_call({:on_connect, conn_id, host}, _from, state) do
    framer = Framer.new()
    framers2 = Map.put(state.framers, conn_id, framer)

    {server2, responses} = IrcServer.on_connect(state.server_state, conn_id, host)
    send_responses(state.loop, responses)

    {:reply, :ok, %{state | server_state: server2, framers: framers2}}
  end

  def handle_call({:on_data, conn_id, data}, _from, state) do
    framer = Map.get(state.framers, conn_id, Framer.new())
    framer2 = Framer.feed(framer, data)
    {framer3, lines} = Framer.frames(framer2)

    {server2, _framers2} =
      Enum.reduce(lines, {state.server_state, state.framers}, fn raw_line, {srv, frmrs} ->
        line = :unicode.characters_to_binary(raw_line, :utf8)

        case IrcProto.parse(line) do
          {:ok, msg} ->
            {srv2, resps} = IrcServer.on_message(srv, conn_id, msg)
            send_responses(state.loop, resps)
            {srv2, frmrs}

          {:error, _} ->
            # Malformed line — skip silently.
            {srv, frmrs}
        end
      end)

    framers3 = Map.put(state.framers, conn_id, framer3)
    {:reply, :ok, %{state | server_state: server2, framers: framers3}}
  end

  def handle_call({:on_disconnect, conn_id}, _from, state) do
    {server2, responses} = IrcServer.on_disconnect(state.server_state, conn_id)
    send_responses(state.loop, responses)
    framers2 = Map.delete(state.framers, conn_id)
    {:reply, :ok, %{state | server_state: server2, framers: framers2}}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  defp send_responses(loop, responses) do
    Enum.each(responses, fn {target_conn_id, msg} ->
      wire = IrcProto.serialize(msg)
      EventLoop.send_to(loop, target_conn_id, wire)
    end)
  end
end
