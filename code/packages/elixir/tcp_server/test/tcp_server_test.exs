defmodule CodingAdventures.TcpServerTest do
  use ExUnit.Case

  alias CodingAdventures.TcpServer
  alias CodingAdventures.TcpServer.Connection

  defp connection do
    %Connection{
      id: 1,
      peer_addr: {"127.0.0.1", 45_001},
      local_addr: {"127.0.0.1", 63_079}
    }
  end

  defp start_server(server) do
    {:ok, started} = TcpServer.start(server)
    task = Task.async(fn -> TcpServer.serve(started) end)
    Process.sleep(50)
    {started, task}
  end

  defp stop_server(server, task) do
    TcpServer.stop(server)
    assert :ok = Task.await(task, 1_000)
  end

  defp send_recv(port, data) do
    {:ok, socket} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, active: false], 1_000)
    :ok = :gen_tcp.send(socket, data)
    result = :gen_tcp.recv(socket, 0, 1_000)
    :gen_tcp.close(socket)
    result
  end

  test "default handler echoes without tcp" do
    server = TcpServer.new(port: 0)
    assert {<<"hello">>, conn} = TcpServer.handle(server, connection(), <<"hello">>)
    assert conn.selected_db == 0
  end

  test "connection-aware handler can update state" do
    server =
      TcpServer.with_handler(fn conn, data ->
        updated = %{conn | read_buffer: conn.read_buffer <> data}

        if byte_size(updated.read_buffer) < 6 do
          {nil, updated}
        else
          {updated.read_buffer, %{updated | read_buffer: <<>>, selected_db: 2}}
        end
      end)

    {empty, conn} = TcpServer.handle(server, connection(), <<"buf">>)
    assert empty == <<>>
    {response, conn} = TcpServer.handle(server, conn, <<"fer">>)
    assert response == <<"buffer">>
    assert conn.selected_db == 2
    assert conn.read_buffer == <<>>
  end

  test "start reports address and stop closes listener" do
    server = TcpServer.new(port: 0, backlog: 0, buffer_size: 0)
    refute TcpServer.running?(server)
    assert TcpServer.address(server) == nil
    refute TcpServer.running?(TcpServer.stop(server))

    {:ok, started} = TcpServer.start(server)
    assert {:ok, ^started} = TcpServer.start(started)
    assert TcpServer.running?(started)
    assert {"127.0.0.1", port} = TcpServer.address(started)
    assert is_integer(port)
    assert String.contains?(to_string(started), "running")

    stopped = TcpServer.stop(started)
    refute TcpServer.running?(stopped)
  end

  test "start falls back to loopback for hostnames" do
    {:ok, started} = TcpServer.start(TcpServer.new(host: "localhost", port: 0))
    assert {"127.0.0.1", port} = TcpServer.address(started)
    assert is_integer(port)
    TcpServer.stop(started)
  end

  test "start maps address in use errors" do
    {:ok, first} = TcpServer.start(TcpServer.new(port: 0))
    {_host, port} = TcpServer.address(first)

    assert {:error, :address_in_use} = TcpServer.start(TcpServer.new(port: port))

    TcpServer.stop(first)
  end

  test "loopback echo request" do
    {server, task} = start_server(TcpServer.new(port: 0))
    {_host, port} = TcpServer.address(server)

    assert {:ok, <<"hello world">>} = send_recv(port, <<"hello world">>)

    stop_server(server, task)
  end

  test "multiple sequential clients use custom handler" do
    {server, task} =
      start_server(
        TcpServer.with_handler(
          fn data ->
            String.upcase(data)
          end,
          port: 0
        )
      )

    {_host, port} = TcpServer.address(server)

    assert {:ok, <<"ONE">>} = send_recv(port, <<"one">>)
    assert {:ok, <<"TWO">>} = send_recv(port, <<"two">>)

    stop_server(server, task)
  end
end
