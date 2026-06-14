defmodule CodingAdventures.IrcServerNative.ServerTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.IrcServerNative.Server

  # ── helpers ────────────────────────────────────────────────────────────────

  defp connect(port) do
    {:ok, sock} =
      :gen_tcp.connect(~c"127.0.0.1", port, [:binary, active: false, packet: :raw], 2000)

    sock
  end

  defp recv_until(sock, needle, deadline \\ nil) do
    deadline = deadline || System.monotonic_time(:millisecond) + 5000
    recv_until(sock, needle, deadline, "")
  end

  defp recv_until(sock, needle, deadline, acc) do
    cond do
      String.contains?(acc, needle) ->
        acc

      System.monotonic_time(:millisecond) >= deadline ->
        acc

      true ->
        case :gen_tcp.recv(sock, 0, 300) do
          {:ok, data} -> recv_until(sock, needle, deadline, acc <> data)
          {:error, :timeout} -> recv_until(sock, needle, deadline, acc)
          {:error, _} -> acc
        end
    end
  end

  defp register(sock, nick) do
    :ok = :gen_tcp.send(sock, "NICK #{nick}\r\nUSER #{nick} 0 * :#{nick}\r\n")
    welcome = recv_until(sock, "001")
    assert String.contains?(welcome, "001"), "expected 001 welcome for #{nick}"
  end

  defp start_server do
    {:ok, server} = Server.new(port: 0, server_name: "irc.test")
    :ok = Server.serve_background(server)
    # wait until running
    Enum.reduce_while(1..200, nil, fn _, _ ->
      if Server.running?(server), do: {:halt, :ok}, else: (Process.sleep(5); {:cont, nil})
    end)

    server
  end

  # ── tests ──────────────────────────────────────────────────────────────────

  test "reports the ephemeral bound address" do
    {:ok, server} = Server.new(port: 0)
    assert Server.local_host(server) == "127.0.0.1"
    assert Server.local_port(server) > 0
    assert Server.local_addr(server) == "127.0.0.1:#{Server.local_port(server)}"
  end

  test "running? flips after serve_background" do
    {:ok, server} = Server.new(port: 0)
    refute Server.running?(server)
    :ok = Server.serve_background(server)
    Enum.reduce_while(1..200, nil, fn _, _ ->
      if Server.running?(server), do: {:halt, :ok}, else: (Process.sleep(5); {:cont, nil})
    end)

    assert Server.running?(server)
    :ok = Server.stop(server)
  end

  test "registration and PING/PONG" do
    server = start_server()
    sock = connect(Server.local_port(server))
    register(sock, "alice")
    :ok = :gen_tcp.send(sock, "PING :liveness\r\n")
    pong = recv_until(sock, "PONG")
    assert String.contains?(pong, "PONG")
    :gen_tcp.close(sock)
    :ok = Server.stop(server)
  end

  test "PRIVMSG broadcasts between two clients" do
    server = start_server()
    alice = connect(Server.local_port(server))
    bob = connect(Server.local_port(server))
    register(alice, "alice")
    register(bob, "bob")
    :ok = :gen_tcp.send(alice, "JOIN #test\r\n")
    :ok = :gen_tcp.send(bob, "JOIN #test\r\n")
    recv_until(alice, "JOIN")
    recv_until(bob, "JOIN")

    # Alice speaks; Bob (a different connection) must receive it — exercises the
    # Rust engine's in-process mailbox fan-out.
    :ok = :gen_tcp.send(alice, "PRIVMSG #test :hello bob\r\n")
    received = recv_until(bob, "hello bob")
    assert String.contains?(received, "PRIVMSG")
    assert String.contains?(received, "hello bob")

    :gen_tcp.close(alice)
    :gen_tcp.close(bob)
    :ok = Server.stop(server)
  end
end
