defmodule CodingAdventures.Conduit.ServerTest do
  @moduledoc """
  E2E tests: spin up real TCP servers, fire HTTP requests with `:httpc`,
  verify responses end-to-end through the Rust NIF bridge.

  Each describe-block starts the server in `setup_all`, fires a few
  requests, then `Server.stop/1` in `on_exit`. Async is OFF because the
  NIF table (slot ID counter) is global state.
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.Conduit.{Application, Server}
  import CodingAdventures.Conduit.HandlerContext

  setup_all do
    {:ok, _} = :application.ensure_all_started(:inets)
    :ok
  end

  defp http_get(port, path, headers \\ []) do
    url = ~c"http://127.0.0.1:#{port}#{path}"
    {:ok, {{_, status, _}, _hdrs, body}} = :httpc.request(:get, {url, headers}, [], [])
    {status, IO.iodata_to_binary(body)}
  end

  defp http_get_full(port, path, headers \\ []) do
    url = ~c"http://127.0.0.1:#{port}#{path}"
    # autoredirect=false so we can test that a 302 is actually returned
    # (otherwise httpc transparently follows it and we never see the redirect).
    opts = [{:autoredirect, false}]

    {:ok, {{_, status, _}, hdrs, body}} =
      :httpc.request(:get, {url, headers}, opts, [])

    {status, hdrs, IO.iodata_to_binary(body)}
  end

  defp http_post(port, path, body, content_type) do
    url = ~c"http://127.0.0.1:#{port}#{path}"

    {:ok, {{_, status, _}, _hdrs, resp}} =
      :httpc.request(:post, {url, [], content_type, body}, [], [])

    {status, IO.iodata_to_binary(resp)}
  end

  defp http_delete(port, path) do
    url = ~c"http://127.0.0.1:#{port}#{path}"
    {:ok, {{_, status, _}, _hdrs, body}} = :httpc.request(:delete, {url, []}, [], [])
    {status, IO.iodata_to_binary(body)}
  end

  describe "basic routes" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> html("<h1>OK</h1>") end)
        |> Application.get("/ping", fn _ -> text("pong") end)
        |> Application.get("/health", fn _ -> json(%{ok: true}) end)
        |> Application.get("/hello/:name", fn req ->
          json(%{message: "Hello " <> req.params["name"]})
        end)
        |> Application.delete("/items/:id", fn req ->
          text("deleted " <> req.params["id"])
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, server: server, port: server.port}
    end

    test "GET / returns 200 HTML", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "<h1>OK</h1>"
    end

    test "GET /ping returns plain pong", %{port: port} do
      {200, body} = http_get(port, "/ping")
      assert body == "pong"
    end

    test "GET /health returns JSON", %{port: port} do
      {200, body} = http_get(port, "/health")
      assert body =~ ~r/\"ok\"\s*:\s*true/
    end

    test "GET /hello/:name captures the route param", %{port: port} do
      {200, body} = http_get(port, "/hello/Adhithya")
      assert body =~ "Hello Adhithya"
    end

    test "DELETE /items/:id captures the route param", %{port: port} do
      {200, body} = http_delete(port, "/items/abc-123")
      assert body == "deleted abc-123"
    end
  end

  describe "post body + json" do
    setup do
      app =
        Application.new()
        |> Application.post("/echo", fn req ->
          # Echo the raw body back (avoids depending on JSON in older Elixir).
          {200, %{"content-type" => req.content_type}, req.body}
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "POST /echo returns the body", %{port: port} do
      {200, body} = http_post(port, "/echo", "hello world", ~c"text/plain")
      assert body == "hello world"
    end
  end

  describe "before filter halt" do
    setup do
      app =
        Application.new()
        |> Application.before_filter(fn req ->
          if req.path == "/down", do: halt(503, "Maintenance")
        end)
        |> Application.get("/", fn _ -> text("up") end)
        |> Application.get("/down", fn _ -> text("never") end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "halt(503) short-circuits", %{port: port} do
      {503, body} = http_get(port, "/down")
      assert body == "Maintenance"
    end

    test "before filter passes through normal requests", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "up"
    end
  end

  describe "redirect + halt helpers" do
    setup do
      app =
        Application.new()
        |> Application.get("/old", fn _ -> redirect("/new") end)
        |> Application.get("/forbidden", fn _ -> halt(403, "no") end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "redirect returns 302 with Location header", %{port: port} do
      {302, hdrs, _} = http_get_full(port, "/old")
      assert Enum.any?(hdrs, fn {name, value} ->
               String.downcase(to_string(name)) == "location" and to_string(value) == "/new"
             end)
    end

    test "halt(403) returns Forbidden", %{port: port} do
      {403, body} = http_get(port, "/forbidden")
      assert body == "no"
    end
  end

  describe "not_found handler" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("home") end)
        |> Application.not_found_handler(fn req ->
          html("<h1>Not Found: #{req.path}</h1>", 404)
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "custom not_found handler runs for unknown paths", %{port: port} do
      {404, body} = http_get(port, "/nope")
      assert body =~ "Not Found: /nope"
    end

    test "known route still works", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "home"
    end
  end

  describe "error handler" do
    setup do
      app =
        Application.new()
        |> Application.get("/boom", fn _ -> raise "boom" end)
        |> Application.error_handler(fn req ->
          # The Rust side encodes the error message as conduit.error in env
          msg = req.env["conduit.error"] || ""
          json(%{error: msg}, 500)
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "exception routes through error_handler with conduit.error available", %{port: port} do
      {500, body} = http_get(port, "/boom")
      assert body =~ "boom"
    end
  end

  describe "query params" do
    setup do
      app =
        Application.new()
        |> Application.get("/search", fn req ->
          json(%{q: req.query_params["q"] || "", n: req.query_params["n"] || ""})
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "are surfaced via req.query_params", %{port: port} do
      {200, body} = http_get(port, "/search?q=hello&n=5")
      assert body =~ "hello"
      assert body =~ "5"
    end
  end

  describe "server metadata" do
    setup do
      app = Application.new() |> Application.get("/", fn _ -> text("ok") end)
      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, server: server}
    end

    test "local_port reports a non-zero bound port", %{server: server} do
      assert Server.local_port(server) > 0
    end

    test "running? toggles after start/stop", %{server: server} do
      assert Server.running?(server)
      Server.stop(server)
      Process.sleep(100)
      refute Server.running?(server)
    end
  end

  describe "after filter" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("hello") end)
        |> Application.after_filter(fn _req ->
          {201, %{"x-modified" => "yes"}, "rewritten"}
        end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Server.serve_background(server)
      Process.sleep(100)
      on_exit(fn -> Server.stop(server) end)
      {:ok, port: server.port}
    end

    test "rewrites the response when it returns a tuple", %{port: port} do
      {201, hdrs, body} = http_get_full(port, "/")
      assert body == "rewritten"

      assert Enum.any?(hdrs, fn {name, value} ->
               String.downcase(to_string(name)) == "x-modified" and to_string(value) == "yes"
             end)
    end
  end
end
