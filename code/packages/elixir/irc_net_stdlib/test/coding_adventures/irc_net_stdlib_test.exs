defmodule CodingAdventures.IrcNetStdlibTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.IrcNetStdlib.EventLoop
  alias CodingAdventures.IrcNetStdlib.Listener

  # ---------------------------------------------------------------------------
  # RecordingHandler records lifecycle callbacks into a per-test Agent.
  # The Agent pid is injected via :persistent_term so worker Tasks can find it.
  # ---------------------------------------------------------------------------

  defmodule RecordingHandler do
    @moduledoc false
    @behaviour CodingAdventures.IrcNetStdlib.Handler

    defp agent do
      key = Process.get(:recording_agent_key)
      :persistent_term.get(key)
    end

    @impl true
    def on_connect(conn_id, host) do
      Agent.update(agent(), fn events -> events ++ [{:connect, conn_id, host}] end)
    end

    @impl true
    def on_data(conn_id, data) do
      Agent.update(agent(), fn events -> events ++ [{:data, conn_id, data}] end)
    end

    @impl true
    def on_disconnect(conn_id) do
      Agent.update(agent(), fn events -> events ++ [{:disconnect, conn_id}] end)
    end
  end

  # InjectingHandler sets :recording_agent_key in each worker process dict
  # before delegating to RecordingHandler.
  defmodule InjectingHandler do
    @moduledoc false
    @behaviour CodingAdventures.IrcNetStdlib.Handler

    @impl true
    def on_connect(conn_id, host) do
      inject_key()
      RecordingHandler.on_connect(conn_id, host)
    end

    @impl true
    def on_data(conn_id, data) do
      inject_key()
      RecordingHandler.on_data(conn_id, data)
    end

    @impl true
    def on_disconnect(conn_id) do
      inject_key()
      RecordingHandler.on_disconnect(conn_id)
    end

    defp inject_key do
      key = :persistent_term.get(:irc_net_stdlib_test_key, nil)
      if key, do: Process.put(:recording_agent_key, key)
    end
  end

  # ---------------------------------------------------------------------------
  # Test setup
  # ---------------------------------------------------------------------------

  setup do
    key = :"test_agent_#{:erlang.unique_integer([:positive])}"
    {:ok, agent} = Agent.start_link(fn -> [] end)
    :persistent_term.put(key, agent)
    :persistent_term.put(:irc_net_stdlib_test_key, key)

    {:ok, loop} = EventLoop.start_link()
    {:ok, sock} = Listener.listen("127.0.0.1", 0)
    port = Listener.port!(sock)
    {:ok, _pid} = EventLoop.run(loop, sock, InjectingHandler)

    on_exit(fn ->
      :persistent_term.erase(:irc_net_stdlib_test_key)
      :persistent_term.erase(key)
      if Process.alive?(loop), do: EventLoop.stop(loop)
    end)

    %{loop: loop, port: port, agent: agent}
  end

  # ---------------------------------------------------------------------------
  # Helper: connect a client socket, run a function, then close.
  # ---------------------------------------------------------------------------

  defp with_client(port, fun) do
    {:ok, client} = :gen_tcp.connect({127, 0, 0, 1}, port,
      [:binary, packet: :raw, active: false])
    try do
      fun.(client)
    after
      :gen_tcp.close(client)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: Listener
  # ---------------------------------------------------------------------------

  describe "Listener.listen/2" do
    test "creates a listening socket on a free port" do
      {:ok, sock} = Listener.listen("127.0.0.1", 0)
      port = Listener.port!(sock)
      assert is_integer(port)
      assert port > 0
      Listener.close(sock)
    end

    test "port!/1 returns the bound port" do
      {:ok, sock} = Listener.listen("127.0.0.1", 0)
      p = Listener.port!(sock)
      assert p > 0 and p <= 65535
      Listener.close(sock)
    end

    test "close/1 closes the socket" do
      {:ok, sock} = Listener.listen("127.0.0.1", 0)
      assert :ok = Listener.close(sock)
      assert {:error, _} = :inet.port(sock)
    end

    test "invalid host raises ArgumentError" do
      assert_raise ArgumentError, fn ->
        Listener.listen("not-a-valid-ip", 0)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: EventLoop.start_link/1
  # ---------------------------------------------------------------------------

  describe "EventLoop.start_link/1" do
    test "starts successfully" do
      {:ok, pid} = EventLoop.start_link()
      assert Process.alive?(pid)
      GenServer.stop(pid)
    end

    test "accepts :name option" do
      name = :"test_event_loop_#{:erlang.unique_integer([:positive])}"
      {:ok, pid} = EventLoop.start_link(name: name)
      assert Process.alive?(pid)
      assert Process.whereis(name) == pid
      GenServer.stop(pid)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: Connection lifecycle
  # ---------------------------------------------------------------------------

  describe "connection lifecycle" do
    test "on_connect fires when a client connects", %{port: port, agent: agent} do
      with_client(port, fn _client ->
        Process.sleep(100)
        events = Agent.get(agent, & &1)
        connect_events = Enum.filter(events, fn {tag, _, _} -> tag == :connect end)
        assert length(connect_events) >= 1
        {_tag, conn_id, host} = hd(connect_events)
        assert is_integer(conn_id)
        assert conn_id >= 1
        assert is_binary(host)
      end)
    end

    test "on_connect assigns unique conn_ids to multiple connections", %{port: port, agent: agent} do
      with_client(port, fn _c1 ->
        with_client(port, fn _c2 ->
          Process.sleep(100)
          events = Agent.get(agent, & &1)
          ids = for {:connect, id, _} <- events, do: id
          assert length(Enum.uniq(ids)) == length(ids)
        end)
      end)
    end

    test "on_data fires when the client sends data", %{port: port, agent: agent} do
      with_client(port, fn client ->
        :gen_tcp.send(client, "NICK alice\r\n")
        Process.sleep(100)
        events = Agent.get(agent, & &1)
        data_events = for {:data, _, _} = e <- events, do: e
        assert length(data_events) >= 1
        {_tag, _cid, data} = hd(data_events)
        assert data == "NICK alice\r\n"
      end)
    end

    test "on_disconnect fires after the client closes", %{port: port, agent: agent} do
      with_client(port, fn _client -> Process.sleep(50) end)
      Process.sleep(300)
      events = Agent.get(agent, & &1)
      disconnect_events = for {:disconnect, _} <- events, do: true
      assert length(disconnect_events) >= 1
    end

    test "multiple data chunks are delivered in order", %{port: port, agent: agent} do
      with_client(port, fn client ->
        :gen_tcp.send(client, "first\r\n")
        :gen_tcp.send(client, "second\r\n")
        Process.sleep(200)
        events = Agent.get(agent, & &1)
        data = for {:data, _, d} <- events, do: d
        combined = Enum.join(data)
        assert String.contains?(combined, "first")
        assert String.contains?(combined, "second")
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: send_to/3
  # ---------------------------------------------------------------------------

  describe "send_to/3" do
    test "sends data to a connected client", %{loop: loop, port: port, agent: agent} do
      with_client(port, fn client ->
        Process.sleep(100)
        events = Agent.get(agent, & &1)
        [{:connect, conn_id, _}] = Enum.filter(events, fn {tag, _, _} -> tag == :connect end)

        EventLoop.send_to(loop, conn_id, "PING :server\r\n")

        {:ok, received} = :gen_tcp.recv(client, 0, 1000)
        assert received == "PING :server\r\n"
      end)
    end

    test "send_to with unknown conn_id is a no-op", %{loop: loop} do
      assert :ok = EventLoop.send_to(loop, 9999, "data")
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: stop/1
  # ---------------------------------------------------------------------------

  describe "stop/1" do
    test "stop/1 returns :ok and clears the loop state", %{loop: loop} do
      assert :ok = EventLoop.stop(loop)
      assert :ok = EventLoop.send_to(loop, 9999, "data")
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: register/deregister conn
  # ---------------------------------------------------------------------------

  describe "register_conn/2 and deregister_conn/2" do
    test "register_conn assigns incremental ids" do
      {:ok, loop} = EventLoop.start_link()
      {:ok, sock1} = Listener.listen("127.0.0.1", 0)
      {:ok, sock2} = Listener.listen("127.0.0.1", 0)

      {:ok, id1} = EventLoop.register_conn(loop, sock1)
      {:ok, id2} = EventLoop.register_conn(loop, sock2)

      assert id1 < id2
      assert id2 == id1 + 1

      EventLoop.deregister_conn(loop, id1)
      EventLoop.deregister_conn(loop, id2)
      Listener.close(sock1)
      Listener.close(sock2)
      GenServer.stop(loop)
    end

    test "deregister_conn removes the entry so send_to becomes a no-op" do
      {:ok, loop} = EventLoop.start_link()
      {:ok, sock} = Listener.listen("127.0.0.1", 0)
      {:ok, id} = EventLoop.register_conn(loop, sock)

      EventLoop.deregister_conn(loop, id)
      assert :ok = EventLoop.send_to(loop, id, "test")

      Listener.close(sock)
      GenServer.stop(loop)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: dispatch/2 serialisation
  # ---------------------------------------------------------------------------

  describe "dispatch/2" do
    test "executes a function inside the GenServer" do
      {:ok, loop} = EventLoop.start_link()
      result = EventLoop.dispatch(loop, fn -> 42 end)
      assert result == 42
      GenServer.stop(loop)
    end

    test "serialises concurrent calls from multiple processes" do
      {:ok, loop} = EventLoop.start_link()
      parent = self()
      n = 20
      for i <- 1..n do
        spawn(fn ->
          EventLoop.dispatch(loop, fn -> send(parent, {:done, i}) end)
        end)
      end
      received = for _ <- 1..n, do: (receive do {:done, i} -> i after 1000 -> nil end)
      assert length(Enum.filter(received, & &1 != nil)) == n
      GenServer.stop(loop)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: integration
  # ---------------------------------------------------------------------------

  describe "integration" do
    test "full connect-send-disconnect cycle", %{loop: loop, port: port, agent: agent} do
      with_client(port, fn client ->
        Process.sleep(100)
        events_before = Agent.get(agent, & &1)
        [{:connect, conn_id, _host}] =
          Enum.filter(events_before, fn {tag, _, _} -> tag == :connect end)

        EventLoop.send_to(loop, conn_id, ":irc.local PING :test\r\n")
        {:ok, data} = :gen_tcp.recv(client, 0, 1000)
        assert data == ":irc.local PING :test\r\n"

        :gen_tcp.send(client, "PONG :test\r\n")
        Process.sleep(150)
        events_after = Agent.get(agent, & &1)
        data_events = for {:data, ^conn_id, d} <- events_after, do: d
        assert Enum.any?(data_events, &String.contains?(&1, "PONG"))
      end)

      Process.sleep(300)
      events_final = Agent.get(agent, & &1)
      disconnect_events = for {:disconnect, _} <- events_final, do: true
      assert length(disconnect_events) >= 1
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: IrcNetStdlib facade
  # ---------------------------------------------------------------------------

  describe "CodingAdventures.IrcNetStdlib facade" do
    test "start_link/0 delegates to EventLoop.start_link/1" do
      {:ok, pid} = CodingAdventures.IrcNetStdlib.start_link()
      assert Process.alive?(pid)
      GenServer.stop(pid)
    end

    test "listen/2 delegates to Listener.listen/2" do
      {:ok, sock} = CodingAdventures.IrcNetStdlib.listen("127.0.0.1", 0)
      port = Listener.port!(sock)
      assert port > 0
      Listener.close(sock)
    end

    test "run/3 delegates to EventLoop.run/3" do
      {:ok, loop} = CodingAdventures.IrcNetStdlib.start_link()
      {:ok, sock} = CodingAdventures.IrcNetStdlib.listen("127.0.0.1", 0)
      port = Listener.port!(sock)
      {:ok, _pid} = CodingAdventures.IrcNetStdlib.run(loop, sock, InjectingHandler)
      {:ok, client} = :gen_tcp.connect({127, 0, 0, 1}, port, [:binary, active: false])
      Process.sleep(50)
      :gen_tcp.close(client)
      CodingAdventures.IrcNetStdlib.stop(loop)
    end

    test "send_to/3 delegates to EventLoop.send_to/3" do
      {:ok, loop} = CodingAdventures.IrcNetStdlib.start_link()
      assert :ok = CodingAdventures.IrcNetStdlib.send_to(loop, 42, "test")
      GenServer.stop(loop)
    end

    test "stop/1 delegates to EventLoop.stop/1" do
      {:ok, loop} = CodingAdventures.IrcNetStdlib.start_link()
      {:ok, sock} = CodingAdventures.IrcNetStdlib.listen("127.0.0.1", 0)
      {:ok, _pid} = CodingAdventures.IrcNetStdlib.run(loop, sock, InjectingHandler)
      assert :ok = CodingAdventures.IrcNetStdlib.stop(loop)
    end
  end
end
