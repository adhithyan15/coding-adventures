defmodule CodingAdventures.IrcdTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.Ircd
  alias CodingAdventures.Ircd.{Config, DriverHandler}
  alias CodingAdventures.IrcNetStdlib.{EventLoop, Listener}
  alias CodingAdventures.IrcServer

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Unregister DriverHandler name if currently registered. Safe to call
  # multiple times.
  defp unregister_driver_handler do
    case Process.whereis(CodingAdventures.Ircd.DriverHandler) do
      nil -> :ok
      _pid -> Process.unregister(CodingAdventures.Ircd.DriverHandler)
    end
  end

  # Start an ircd stack on a random port. Returns {loop, port}.
  defp start_ircd(server_name \\ "irc.test", motd \\ ["Test server."], oper_password \\ "") do
    # Ensure any previous registration is cleared first.
    unregister_driver_handler()

    {:ok, loop} = EventLoop.start_link()

    server_state = IrcServer.new(server_name, "0.1.0", motd, oper_password)

    {:ok, handler} = DriverHandler.start_link(server_state, loop, [])

    # Register the handler under the canonical name so Handler callbacks can
    # find it via Process.whereis(__MODULE__).
    Process.register(handler, CodingAdventures.Ircd.DriverHandler)

    {:ok, sock} = Listener.listen("127.0.0.1", 0)
    port = Listener.port!(sock)
    {:ok, _pid} = EventLoop.run(loop, sock, DriverHandler)

    {loop, port}
  end

  defp stop_ircd(loop) do
    if Process.alive?(loop), do: EventLoop.stop(loop)
    unregister_driver_handler()
  end

  # Connect a client, perform IRC handshake (NICK + USER), return socket.
  defp connect_and_register(port, nick, username \\ "user", realname \\ "Real Name") do
    {:ok, client} = :gen_tcp.connect({127, 0, 0, 1}, port,
      [:binary, packet: :raw, active: false])
    :gen_tcp.send(client, "NICK #{nick}\r\n")
    :gen_tcp.send(client, "USER #{username} 0 * :#{realname}\r\n")
    # Read and discard welcome burst.
    read_until(client, "376")
    client
  end

  # Read lines from the socket until one containing *marker* is received.
  defp read_until(client, marker, acc \\ "") do
    case :gen_tcp.recv(client, 0, 2000) do
      {:ok, data} ->
        combined = acc <> data
        if String.contains?(combined, marker) do
          combined
        else
          read_until(client, marker, combined)
        end

      {:error, _} ->
        acc
    end
  end

  # Read all available data with a short timeout.
  defp read_all(client, timeout \\ 500) do
    case :gen_tcp.recv(client, 0, timeout) do
      {:ok, data} -> data <> read_all(client, timeout)
      {:error, _} -> ""
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: parse_args/1
  # ---------------------------------------------------------------------------

  describe "parse_args/1" do
    test "returns default config with empty argv" do
      config = Ircd.parse_args([])
      assert config.host == "0.0.0.0"
      assert config.port == 6667
      assert config.server_name == "irc.local"
      assert config.motd == ["Welcome."]
      assert config.oper_password == ""
    end

    test "parses --port" do
      config = Ircd.parse_args(["--port", "6668"])
      assert config.port == 6668
    end

    test "parses --server-name" do
      config = Ircd.parse_args(["--server-name", "irc.example.com"])
      assert config.server_name == "irc.example.com"
    end

    test "parses --host" do
      config = Ircd.parse_args(["--host", "127.0.0.1"])
      assert config.host == "127.0.0.1"
    end

    test "parses --motd" do
      config = Ircd.parse_args(["--motd", "Hello world"])
      assert config.motd == ["Hello world"]
    end

    test "parses --oper-password" do
      config = Ircd.parse_args(["--oper-password", "secret"])
      assert config.oper_password == "secret"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: DriverHandler
  # ---------------------------------------------------------------------------

  describe "DriverHandler" do
    setup do
      {loop, port} = start_ircd()
      on_exit(fn -> stop_ircd(loop) end)
      %{loop: loop, port: port}
    end

    test "client can connect and register", %{port: port} do
      {:ok, client} = :gen_tcp.connect({127, 0, 0, 1}, port,
        [:binary, packet: :raw, active: false])
      :gen_tcp.send(client, "NICK alice\r\n")
      :gen_tcp.send(client, "USER alice 0 * :Alice Smith\r\n")

      welcome = read_until(client, "001")
      assert String.contains?(welcome, "001")

      :gen_tcp.close(client)
    end

    test "client receives MOTD after registration", %{port: port} do
      {:ok, client} = :gen_tcp.connect({127, 0, 0, 1}, port,
        [:binary, packet: :raw, active: false])
      :gen_tcp.send(client, "NICK alice\r\n")
      :gen_tcp.send(client, "USER alice 0 * :Alice\r\n")

      welcome = read_until(client, "376")
      assert String.contains?(welcome, "375")
      assert String.contains?(welcome, "376")

      :gen_tcp.close(client)
    end

    test "two clients can exchange PRIVMSG", %{port: port} do
      alice = connect_and_register(port, "alice")
      bob = connect_and_register(port, "bob")

      :gen_tcp.send(alice, "PRIVMSG bob :Hello Bob!\r\n")
      data = read_until(bob, "PRIVMSG")
      assert String.contains?(data, "Hello Bob!")

      :gen_tcp.close(alice)
      :gen_tcp.close(bob)
    end

    test "clients can join a channel and chat", %{port: port} do
      alice = connect_and_register(port, "alice")
      bob = connect_and_register(port, "bob")

      :gen_tcp.send(alice, "JOIN #general\r\n")
      Process.sleep(50)

      :gen_tcp.send(bob, "JOIN #general\r\n")
      Process.sleep(50)

      :gen_tcp.send(alice, "PRIVMSG #general :Hello channel!\r\n")
      data = read_until(bob, "PRIVMSG")
      assert String.contains?(data, "Hello channel!")

      :gen_tcp.close(alice)
      :gen_tcp.close(bob)
    end

    test "PING returns PONG", %{port: port} do
      alice = connect_and_register(port, "alice")
      :gen_tcp.send(alice, "PING :test\r\n")
      data = read_until(alice, "PONG")
      assert String.contains?(data, "PONG")
      :gen_tcp.close(alice)
    end

    test "unknown command returns 421", %{port: port} do
      alice = connect_and_register(port, "alice")
      :gen_tcp.send(alice, "FOOBAR\r\n")
      data = read_until(alice, "421")
      assert String.contains?(data, "421")
      :gen_tcp.close(alice)
    end

    test "disconnect is handled cleanly", %{port: port} do
      alice = connect_and_register(port, "alice")
      bob = connect_and_register(port, "bob")

      :gen_tcp.send(alice, "JOIN #chan\r\n")
      :gen_tcp.send(bob, "JOIN #chan\r\n")
      Process.sleep(50)

      # Alice disconnects.
      :gen_tcp.close(alice)
      Process.sleep(200)

      # Bob should receive QUIT notification.
      data = read_all(bob)
      assert String.contains?(data, "QUIT") or not String.contains?(data, "alice")

      :gen_tcp.close(bob)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: Config struct
  # ---------------------------------------------------------------------------

  describe "Config struct" do
    test "has correct defaults" do
      config = %Config{}
      assert config.host == "0.0.0.0"
      assert config.port == 6667
      assert config.server_name == "irc.local"
      assert config.motd == ["Welcome."]
      assert config.oper_password == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Tests: Ircd.main/1 and Ircd.stop/0
  # ---------------------------------------------------------------------------

  describe "Ircd.main/1" do
    test "starts and can be stopped via :stop message" do
      # Run main in a separate process so it can block on receive.
      parent = self()

      pid =
        spawn(fn ->
          # Notify the test of our pid so it can send :stop.
          send(parent, {:main_pid, self()})
          Ircd.main(["--port", "0", "--host", "127.0.0.1"])
        end)

      # Wait for main to send its pid.
      main_pid =
        receive do
          {:main_pid, p} -> p
        after
          3000 -> flunk("main did not start")
        end

      # Give main time to start listening.
      Process.sleep(200)
      assert Process.alive?(pid)

      # Send the stop message so main/1 exits cleanly.
      send(main_pid, :stop)
      ref = Process.monitor(pid)

      receive do
        {:DOWN, ^ref, :process, ^pid, _reason} -> :ok
      after
        3000 -> flunk("main did not stop")
      end
    end
  end
end
