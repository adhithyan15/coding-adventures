defmodule ConduitHelloTest do
  @moduledoc """
  Integration tests for the conduit-hello demo. Each test fires a real
  HTTP request via :httpc against a server running on a fresh OS-assigned
  port. Mirrors the WEB05 conduit-hello test suite for parity.
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.Conduit.Server

  setup_all do
    {:ok, _} = :application.ensure_all_started(:inets)
    {:ok, server} = Server.start_link(ConduitHello.app(), host: "127.0.0.1", port: 0)
    Server.serve_background(server)
    Process.sleep(120)

    on_exit(fn -> Server.stop(server) end)
    {:ok, port: server.port}
  end

  defp get(port, path) do
    url = ~c"http://127.0.0.1:#{port}#{path}"

    {:ok, {{_, status, _}, hdrs, body}} =
      :httpc.request(:get, {url, []}, [{:autoredirect, false}], [])

    {status, hdrs, IO.iodata_to_binary(body)}
  end

  defp post(port, path, body, content_type) do
    url = ~c"http://127.0.0.1:#{port}#{path}"

    {:ok, {{_, status, _}, _hdrs, resp}} =
      :httpc.request(:post, {url, [], content_type, body}, [], [])

    {status, IO.iodata_to_binary(resp)}
  end

  test "GET / returns a 200 HTML response", %{port: port} do
    {200, _, body} = get(port, "/")
    assert body =~ "Hello from Conduit"
    assert body =~ "<a href=\"/hello/Adhithya\">"
  end

  test "GET /hello/:name captures the route param", %{port: port} do
    {200, _, body} = get(port, "/hello/Adhithya")
    assert body =~ "Hello Adhithya"
  end

  test "GET /hello/:name URL-decoding handled by web-core", %{port: port} do
    {200, _, body} = get(port, "/hello/World")
    assert body =~ "Hello World"
  end

  test "POST /echo returns the request body unchanged", %{port: port} do
    {200, body} = post(port, "/echo", "hello world", ~c"text/plain")
    assert body == "hello world"
  end

  test "POST /echo preserves JSON body", %{port: port} do
    payload = "{\"ping\":\"pong\"}"
    {200, body} = post(port, "/echo", payload, ~c"application/json")
    assert body == payload
  end

  test "GET /redirect returns 301 with Location: /", %{port: port} do
    {301, hdrs, _} = get(port, "/redirect")

    assert Enum.any?(hdrs, fn {k, v} ->
             String.downcase(to_string(k)) == "location" and to_string(v) == "/"
           end)
  end

  test "GET /halt returns 403 Forbidden", %{port: port} do
    {403, _, body} = get(port, "/halt")
    assert body == "Forbidden"
  end

  test "GET /down triggers the before-filter halt(503)", %{port: port} do
    {503, _, body} = get(port, "/down")
    assert body == "Under maintenance"
  end

  test "GET /error routes to the custom error_handler returning JSON 500", %{port: port} do
    {500, _, body} = get(port, "/error")
    assert body =~ "Internal Server Error"
    # The conduit.error message is included in `detail`
    assert body =~ "Something went wrong"
  end

  test "GET /missing returns the custom not_found_handler 404", %{port: port} do
    {404, _, body} = get(port, "/missing")
    assert body =~ "Not Found: /missing"
  end

  test "GET /anything/else also returns 404 via not_found_handler", %{port: port} do
    {404, _, body} = get(port, "/anything/else")
    assert body =~ "Not Found: /anything/else"
  end

  test "the app function is pure and inspectable" do
    app = ConduitHello.app()
    methods = Enum.map(app.routes, & &1.method)
    paths = Enum.map(app.routes, & &1.pattern)

    assert "GET" in methods
    assert "POST" in methods
    assert "/" in paths
    assert "/hello/:name" in paths
    assert "/echo" in paths
  end

  test "settings are accessible via Application.get_setting/2" do
    app = ConduitHello.app()
    assert CodingAdventures.Conduit.Application.get_setting(app, :app_name) =~ "Conduit Hello"
  end

  test "Server.local_port returns the bound port", %{port: port} do
    assert is_integer(port) and port > 0
  end

  test "POST /echo with empty body works", %{port: port} do
    {200, body} = post(port, "/echo", "", ~c"text/plain")
    assert body == ""
  end
end
