defmodule CodingAdventures.ConduitOtp.WorkerTest do
  @moduledoc """
  Tests for Worker internals that are hard to reach via E2E HTTP tests.

  We use raw TCP sockets to exercise specific code paths in the Worker.
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.ConduitOtp.{Application, Server}
  import CodingAdventures.ConduitOtp.HandlerContext

  setup_all do
    {:ok, _} = :application.ensure_all_started(:inets)
    :ok
  end

  defp start_server(app) do
    {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
    Process.sleep(50)
    on_exit(fn -> Server.stop(server) end)
    server
  end

  defp raw_request(port, request_bin) do
    {:ok, sock} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, {:active, false}])
    :ok = :gen_tcp.send(sock, request_bin)
    Process.sleep(100)

    response =
      case :gen_tcp.recv(sock, 0, 2000) do
        {:ok, data} -> data
        {:error, _} -> ""
      end

    :gen_tcp.close(sock)
    response
  end

  describe "connection: close" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("hello") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "server closes connection when client sends Connection: close", %{port: port} do
      request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "HTTP/1.1 200"
      assert response =~ "hello"
    end
  end

  describe "handler returns nil" do
    # A before_filter that returns nil passes through — the route handler runs.
    setup do
      app =
        Application.new()
        |> Application.before_filter(fn _req -> nil end)
        |> Application.get("/", fn _ -> text("passed through") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "nil-returning filter lets request pass through", %{port: port} do
      request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "passed through"
    end
  end

  describe "throw in before_filter" do
    setup do
      app =
        Application.new()
        |> Application.before_filter(fn _req -> halt(401, "Unauthorized") end)
        |> Application.get("/secret", fn _ -> text("secret data") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "before_filter halt prevents route from running", %{port: port} do
      request = "GET /secret HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "401"
      assert response =~ "Unauthorized"
    end
  end

  describe "after_filter returning nil" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("original") end)
        |> Application.after_filter(fn _req -> nil end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "nil-returning after_filter preserves original response", %{port: port} do
      request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "original"
    end
  end

  describe "not_found via direct TCP" do
    setup do
      app = Application.new() |> Application.get("/only", fn _ -> text("ok") end)
      server = start_server(app)
      {:ok, port: server.port}
    end

    test "returns 404 for missing route", %{port: port} do
      request = "GET /missing HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "404"
    end
  end

  describe "error recovery" do
    setup do
      app =
        Application.new()
        |> Application.get("/crash", fn _ -> raise "deliberate crash" end)
        |> Application.get("/ok", fn _ -> text("ok") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "server continues serving after a handler crash", %{port: port} do
      # First request crashes a worker
      request1 = "GET /crash HTTP/1.1\r\nHost: localhost\r\n\r\n"
      resp1 = raw_request(port, request1)
      assert resp1 =~ "500"

      # Brief pause then second request from a new connection should work fine
      Process.sleep(100)
      request2 = "GET /ok HTTP/1.1\r\nHost: localhost\r\n\r\n"
      resp2 = raw_request(port, request2)
      assert resp2 =~ "ok"
    end
  end

  describe "abrupt client disconnect" do
    setup do
      app = Application.new() |> Application.get("/", fn _ -> text("ok") end)
      server = start_server(app)
      {:ok, port: server.port}
    end

    test "server handles client closing connection without sending a request", %{port: port} do
      # Connect and immediately close — Worker should exit cleanly
      {:ok, sock} = :gen_tcp.connect(~c"127.0.0.1", port, [:binary, {:active, false}])
      :gen_tcp.close(sock)
      # Give worker time to exit
      Process.sleep(100)

      # Server should still be accepting new connections
      request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "ok"
    end
  end

  describe "handler with throw (not conduit_halt)" do
    setup do
      app =
        Application.new()
        |> Application.get("/throw", fn _ -> throw(:unexpected) end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "non-conduit throw results in 500", %{port: port} do
      request = "GET /throw HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "500"
    end
  end

  describe "respond helper" do
    setup do
      app =
        Application.new()
        |> Application.get("/respond", fn _ ->
          {204, %{"x-empty" => "yes"}, ""}
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "204 No Content response is sent correctly", %{port: port} do
      request = "GET /respond HTTP/1.1\r\nHost: localhost\r\n\r\n"
      response = raw_request(port, request)
      assert response =~ "204"
    end
  end

  describe "POST with body" do
    setup do
      app =
        Application.new()
        |> Application.post("/data", fn req ->
          text("got: " <> req.body)
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "POST body is read and available to handler", %{port: port} do
      body = "hello from test"
      len = byte_size(body)
      request =
        "POST /data HTTP/1.1\r\n" <>
          "Host: localhost\r\n" <>
          "Content-Length: #{len}\r\n" <>
          "Content-Type: text/plain\r\n" <>
          "\r\n" <>
          body

      response = raw_request(port, request)
      assert response =~ "got: hello from test"
    end
  end
end
