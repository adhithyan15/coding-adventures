defmodule CodingAdventures.TcpClientTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.TcpClient

  # ===========================================================================
  # Test helpers — ephemeral servers on port 0
  # ===========================================================================
  #
  # Each helper starts a TCP listener on port 0, which tells the OS to assign
  # an unused ephemeral port. This avoids port conflicts when tests run in
  # parallel. The server runs in a separate Task and handles exactly one
  # connection.

  # -- Echo server -----------------------------------------------------------
  #
  # Reads data from the client and sends it straight back. This is the
  # simplest possible TCP server — perfect for verifying that our client
  # can send and receive data correctly.
  #
  #   Client: "Hello\n"  -->  Server
  #   Client: "Hello\n"  <--  Server  (echoed back)

  defp start_echo_server do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, assigned_port} = :inet.port(listener)

    task =
      Task.async(fn ->
        {:ok, client_socket} = :gen_tcp.accept(listener, 5000)
        echo_loop(client_socket)
        :gen_tcp.close(listener)
      end)

    # Brief pause to ensure the listener is ready before tests connect.
    Process.sleep(50)
    {assigned_port, task}
  end

  defp echo_loop(socket) do
    case :gen_tcp.recv(socket, 0, 5000) do
      {:ok, data} ->
        :gen_tcp.send(socket, data)
        echo_loop(socket)

      {:error, :closed} ->
        :ok

      {:error, _reason} ->
        :ok
    end
  end

  # -- Silent server ---------------------------------------------------------
  #
  # Accepts a connection but never sends any data. Used to test read timeouts:
  # the client connects, tries to read, and should get a :timeout error after
  # the configured read_timeout elapses.

  defp start_silent_server do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, assigned_port} = :inet.port(listener)

    task =
      Task.async(fn ->
        {:ok, _client_socket} = :gen_tcp.accept(listener, 5000)
        # Hold the connection open but never send anything.
        # Wait for the test to finish (the task will be awaited or killed).
        Process.sleep(10_000)
        :gen_tcp.close(listener)
      end)

    Process.sleep(50)
    {assigned_port, task}
  end

  # -- Partial server --------------------------------------------------------
  #
  # Sends exactly the given data, then closes the connection. Used to test
  # EOF handling: what happens when the server sends fewer bytes than the
  # client expects?

  defp start_partial_server(data_to_send) do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, assigned_port} = :inet.port(listener)

    task =
      Task.async(fn ->
        {:ok, client_socket} = :gen_tcp.accept(listener, 5000)
        :gen_tcp.send(client_socket, data_to_send)
        # Small delay so the client can read before we close.
        Process.sleep(100)
        :gen_tcp.close(client_socket)
        :gen_tcp.close(listener)
      end)

    Process.sleep(50)
    {assigned_port, task}
  end

  # -- Request-response server -----------------------------------------------
  #
  # Reads one chunk from the client, then sends a pre-configured response.
  # Simulates an HTTP-like exchange: client sends a request, server sends
  # headers + body, then closes.

  defp start_request_response_server(response_data) do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, assigned_port} = :inet.port(listener)

    task =
      Task.async(fn ->
        {:ok, client_socket} = :gen_tcp.accept(listener, 5000)
        # Read the client's request (we don't care about contents).
        _request = :gen_tcp.recv(client_socket, 0, 5000)
        # Send the response.
        :gen_tcp.send(client_socket, response_data)
        Process.sleep(100)
        :gen_tcp.close(client_socket)
        :gen_tcp.close(listener)
      end)

    Process.sleep(50)
    {assigned_port, task}
  end

  # -- Half-close server -----------------------------------------------------
  #
  # Reads until EOF (client shut down write), then sends a final response.
  # Used to test the shutdown_write/close pattern where the client says
  # "I'm done writing" and then reads a final reply.

  defp start_half_close_server(final_response) do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, assigned_port} = :inet.port(listener)

    task =
      Task.async(fn ->
        {:ok, client_socket} = :gen_tcp.accept(listener, 5000)
        received = read_until_eof(client_socket, <<>>)
        :gen_tcp.send(client_socket, final_response)
        # Keep socket open long enough for client to read the response.
        Process.sleep(500)
        :gen_tcp.close(client_socket)
        :gen_tcp.close(listener)
        received
      end)

    Process.sleep(50)
    {assigned_port, task}
  end

  defp read_until_eof(socket, acc) do
    case :gen_tcp.recv(socket, 0, 5000) do
      {:ok, data} -> read_until_eof(socket, acc <> data)
      {:error, :closed} -> acc
      {:error, _} -> acc
    end
  end

  # Shorthand for test connection options with short timeouts.
  defp test_opts, do: [connect_timeout: 5000, read_timeout: 5000, write_timeout: 5000, buffer_size: 4096]

  # ===========================================================================
  # Group 1: Connection lifecycle
  # ===========================================================================

  @tag timeout: 10_000
  test "connect and disconnect" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Verify we got a valid connection struct.
    assert %TcpClient{} = conn
    assert conn.buffer == <<>>
    assert conn.read_timeout == 5000

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "connect with default options" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port)

    # Defaults should match the module constants.
    assert conn.read_timeout == 30_000
    assert conn.write_timeout == 30_000
    assert conn.buffer_size == 8192

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 2: Echo server — basic I/O
  # ===========================================================================

  @tag timeout: 10_000
  test "write and read back exact bytes" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "Hello, TCP!")
    :ok = TcpClient.flush(conn)

    {:ok, {data, _conn2}} = TcpClient.read_exact(conn, 11)
    assert data == "Hello, TCP!"

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "read_line from echo server" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "Hello\r\nWorld\r\n")

    # read_line should return the first line including the newline.
    {:ok, {line1, conn2}} = TcpClient.read_line(conn)
    assert line1 == "Hello\r\n"

    # The second line comes from the updated connection (with leftover buffer).
    {:ok, {line2, conn3}} = TcpClient.read_line(conn2)
    assert line2 == "World\r\n"

    TcpClient.close(conn3)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "read_exact from echo server" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Send 100 bytes of sequential data.
    data = :binary.list_to_bin(Enum.map(0..99, fn i -> rem(i, 256) end))
    :ok = TcpClient.write_all(conn, data)

    {:ok, {result, conn2}} = TcpClient.read_exact(conn, 100)
    assert result == data

    TcpClient.close(conn2)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "read_until with null delimiter" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "key:value\0next")

    {:ok, {result, conn2}} = TcpClient.read_until(conn, 0)
    assert result == "key:value\0"

    # The leftover "next" should still be readable.
    {:ok, {leftover, conn3}} = TcpClient.read_exact(conn2, 4)
    assert leftover == "next"

    TcpClient.close(conn3)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "large data transfer (64 KiB)" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # 64 KiB of sequential bytes.
    data = :binary.list_to_bin(Enum.map(0..65535, fn i -> rem(i, 256) end))
    :ok = TcpClient.write_all(conn, data)

    {:ok, {result, conn2}} = TcpClient.read_exact(conn, 65536)
    assert byte_size(result) == 65536
    assert result == data

    TcpClient.close(conn2)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "multiple request-response exchanges" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Exchange 1: ping
    :ok = TcpClient.write_all(conn, "ping\n")
    {:ok, {line1, conn2}} = TcpClient.read_line(conn)
    assert line1 == "ping\n"

    # Exchange 2: pong (using updated connection for buffer continuity)
    :ok = TcpClient.write_all(conn2, "pong\n")
    {:ok, {line2, conn3}} = TcpClient.read_line(conn2)
    assert line2 == "pong\n"

    TcpClient.close(conn3)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "zero byte write succeeds" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Writing zero bytes should not error.
    assert :ok == TcpClient.write_all(conn, "")

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 3: Timeouts
  # ===========================================================================

  @tag timeout: 15_000
  test "read timeout on silent server" do
    {port, task} = start_silent_server()

    # Use a very short read timeout so the test doesn't take forever.
    short_opts = [connect_timeout: 5000, read_timeout: 500, write_timeout: 5000]
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, short_opts)

    # The silent server never sends data, so read_line should timeout.
    result = TcpClient.read_line(conn)
    assert {:error, :timeout} = result

    TcpClient.close(conn)
    Task.shutdown(task, :brutal_kill)
  end

  @tag timeout: 15_000
  test "read_exact timeout on silent server" do
    {port, task} = start_silent_server()

    short_opts = [connect_timeout: 5000, read_timeout: 500, write_timeout: 5000]
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, short_opts)

    result = TcpClient.read_exact(conn, 100)
    assert {:error, :timeout} = result

    TcpClient.close(conn)
    Task.shutdown(task, :brutal_kill)
  end

  @tag timeout: 15_000
  test "read_until timeout on silent server" do
    {port, task} = start_silent_server()

    short_opts = [connect_timeout: 5000, read_timeout: 500, write_timeout: 5000]
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, short_opts)

    result = TcpClient.read_until(conn, ?\n)
    assert {:error, :timeout} = result

    TcpClient.close(conn)
    Task.shutdown(task, :brutal_kill)
  end

  # ===========================================================================
  # Group 4: Error conditions
  # ===========================================================================

  @tag timeout: 10_000
  test "connection refused on closed port" do
    # Bind to a port, then immediately close the listener.
    # This guarantees no server is listening on that port.
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, reuseaddr: true])
    {:ok, closed_port} = :inet.port(listener)
    :gen_tcp.close(listener)

    result = TcpClient.connect("127.0.0.1", closed_port, test_opts())
    assert {:error, :connection_refused} = result
  end

  @tag timeout: 10_000
  test "DNS resolution failure" do
    result = TcpClient.connect("this.host.does.not.exist.example", 80, test_opts())

    # Either DNS fails (nxdomain) or some ISPs hijack the response.
    case result do
      {:error, :dns_resolution_failed} -> :ok
      {:error, :connection_refused} -> :ok
      {:error, :timeout} -> :ok
      {:error, {:unknown, _}} -> :ok
      other -> flunk("expected DNS failure, got: #{inspect(other)}")
    end
  end

  @tag timeout: 10_000
  test "unexpected EOF when server sends fewer bytes than expected" do
    # Server sends 50 bytes then closes.
    partial_data = :binary.list_to_bin(Enum.to_list(0..49))
    {port, task} = start_partial_server(partial_data)
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Wait for the server to send data and close.
    Process.sleep(200)

    # Try to read 100 bytes — should fail because only 50 are available.
    result = TcpClient.read_exact(conn, 100)
    assert {:error, :unexpected_eof} = result

    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "write to closed connection returns error" do
    # Server sends nothing and closes immediately.
    {port, task} = start_partial_server(<<>>)
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Wait for server to close its end.
    Process.sleep(300)

    # Try multiple large writes — eventually one should fail with
    # :broken_pipe or :connection_reset.
    errors =
      Enum.reduce_while(1..10, :ok, fn _i, _acc ->
        big_data = :binary.copy(<<0>>, 65536)

        case TcpClient.write_all(conn, big_data) do
          :ok ->
            Process.sleep(50)
            {:cont, :ok}

          {:error, reason} ->
            {:halt, {:error, reason}}
        end
      end)

    # We should have gotten an error at some point.
    assert match?({:error, _}, errors), "expected write error after server closed"

    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 5: Half-close (shutdown_write)
  # ===========================================================================

  @tag timeout: 10_000
  test "shutdown_write prevents further writes" do
    # On Windows, :gen_tcp half-close fully closes the socket for both
    # directions at the OTP level. So instead of testing the full
    # half-close pattern (write, shutdown, read response), we verify
    # that shutdown_write succeeds and subsequent writes fail.
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Write some data before shutting down.
    :ok = TcpClient.write_all(conn, "before shutdown\n")

    # Shutdown the write half.
    :ok = TcpClient.shutdown_write(conn)

    # Subsequent writes should fail.
    result = TcpClient.write_all(conn, "after shutdown\n")
    assert {:error, _reason} = result

    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "half-close server receives all data before shutdown" do
    # Verify the server can read everything the client sent before
    # the client called shutdown_write.
    {port, task} = start_half_close_server("DONE\n")
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "request data")
    :ok = TcpClient.shutdown_write(conn)

    # The server should have received our data.
    server_received = Task.await(task, 5000)
    assert server_received == "request data"
  end

  # ===========================================================================
  # Group 6: EOF handling
  # ===========================================================================

  @tag timeout: 10_000
  test "read_line returns empty string at EOF" do
    {port, task} = start_partial_server("hello\n")
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Wait for server to send and close.
    Process.sleep(200)

    # First read: the line.
    {:ok, {line, conn2}} = TcpClient.read_line(conn)
    assert line == "hello\n"

    # Second read: should be EOF (empty string).
    {:ok, {eof_line, _conn3}} = TcpClient.read_line(conn2)
    assert eof_line == ""

    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "read_line returns partial data at EOF without newline" do
    # Server sends data without a trailing newline, then closes.
    {port, task} = start_partial_server("no newline here")
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    Process.sleep(200)

    # Should return whatever was buffered, even without '\n'.
    {:ok, {partial, _conn2}} = TcpClient.read_line(conn)
    assert partial == "no newline here"

    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 7: Address inspection
  # ===========================================================================

  @tag timeout: 10_000
  test "peer_addr returns server address and port" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    {:ok, {addr, peer_port}} = TcpClient.peer_addr(conn)
    assert addr == {127, 0, 0, 1}
    assert peer_port == port

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "local_addr returns client address and ephemeral port" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    {:ok, {addr, local_port}} = TcpClient.local_addr(conn)
    assert addr == {127, 0, 0, 1}
    assert local_port > 0
    # The local port should be different from the server port (ephemeral).
    assert local_port != port

    TcpClient.close(conn)
    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 8: HTTP-like request-response pattern
  # ===========================================================================

  @tag timeout: 10_000
  test "HTTP-like request-response with mixed read methods" do
    response_data = "HTTP/1.0 200 OK\r\nContent-Length: 5\r\n\r\nhello"
    {port, task} = start_request_response_server(response_data)
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # Send a request.
    :ok = TcpClient.write_all(conn, "GET / HTTP/1.0\r\n\r\n")

    # Read response headers line by line.
    {:ok, {status, conn2}} = TcpClient.read_line(conn)
    assert String.starts_with?(status, "HTTP/1.0 200")

    {:ok, {header, conn3}} = TcpClient.read_line(conn2)
    assert String.starts_with?(header, "Content-Length:")

    {:ok, {blank, conn4}} = TcpClient.read_line(conn3)
    assert blank == "\r\n"

    # Read body with read_exact (5 bytes as specified by Content-Length).
    {:ok, {body, _conn5}} = TcpClient.read_exact(conn4, 5)
    assert body == "hello"

    Task.await(task, 5000)
  end

  # ===========================================================================
  # Group 9: Buffer threading (functional state)
  # ===========================================================================

  @tag timeout: 10_000
  test "buffer correctly threads through multiple reads" do
    # Send two complete lines in a single write. The echo server sends
    # them back as one chunk (or possibly two). Our buffer should handle
    # splitting correctly regardless.
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "aaa\nbbb\nccc\n")

    {:ok, {l1, c2}} = TcpClient.read_line(conn)
    {:ok, {l2, c3}} = TcpClient.read_line(c2)
    {:ok, {l3, _c4}} = TcpClient.read_line(c3)

    assert l1 == "aaa\n"
    assert l2 == "bbb\n"
    assert l3 == "ccc\n"

    TcpClient.close(c3)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "read_until with newline delimiter behaves like read_line" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    :ok = TcpClient.write_all(conn, "hello\nworld\n")

    {:ok, {result, conn2}} = TcpClient.read_until(conn, ?\n)
    assert result == "hello\n"

    TcpClient.close(conn2)
    Task.await(task, 5000)
  end

  @tag timeout: 10_000
  test "close is idempotent (second close does not crash)" do
    {port, task} = start_echo_server()
    {:ok, conn} = TcpClient.connect("127.0.0.1", port, test_opts())

    # First close should succeed.
    assert :ok = TcpClient.close(conn)

    # Second close: the socket is already closed. :gen_tcp.close returns :ok
    # even on an already-closed socket in most OTP versions.
    TcpClient.close(conn)

    Task.await(task, 5000)
  end
end
