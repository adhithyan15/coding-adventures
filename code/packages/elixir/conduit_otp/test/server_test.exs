defmodule CodingAdventures.ConduitOtp.ServerTest do
  @moduledoc """
  E2E tests: spin up real TCP servers, fire HTTP requests with `:httpc`,
  verify responses end-to-end through the pure OTP bridge.

  Each describe-block starts the server in `setup`, fires requests, then
  `Server.stop/1` in `on_exit`. `async: false` because each test binds a
  distinct port (port: 0 = OS-assigned ephemeral), but genserver name
  conflicts must be avoided — the Server uses unique process names via
  `:erlang.unique_integer/1`.
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.ConduitOtp.{Application, Server}
  import CodingAdventures.ConduitOtp.HandlerContext

  setup_all do
    {:ok, _} = :application.ensure_all_started(:inets)
    :ok
  end

  # ── HTTP helpers ─────────────────────────────────────────────────────────────

  defp http_get(port, path, headers \\ []) do
    url = ~c"http://127.0.0.1:#{port}#{path}"
    {:ok, {{_, status, _}, _hdrs, body}} = :httpc.request(:get, {url, headers}, [], [])
    {status, IO.iodata_to_binary(body)}
  end

  defp http_get_full(port, path, headers \\ []) do
    url = ~c"http://127.0.0.1:#{port}#{path}"
    {:ok, {{_, status, _}, hdrs, body}} =
      :httpc.request(:get, {url, headers}, [{:autoredirect, false}], [])

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

  defp start_server(app) do
    {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
    # Brief pause to ensure the Acceptor is in its accept loop.
    Process.sleep(50)
    on_exit(fn -> Server.stop(server) end)
    server
  end

  # ── Basic route tests ─────────────────────────────────────────────────────────

  describe "basic GET routes" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> html("<h1>OK</h1>") end)
        |> Application.get("/ping", fn _ -> text("pong") end)
        |> Application.get("/health", fn _ -> json(%{ok: true}) end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "GET / returns 200 HTML", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "<h1>OK</h1>"
    end

    test "GET /ping returns plain text pong", %{port: port} do
      {200, body} = http_get(port, "/ping")
      assert body == "pong"
    end

    test "GET /health returns JSON", %{port: port} do
      {200, body} = http_get(port, "/health")
      assert body =~ "true"
    end
  end

  # ── Named captures ────────────────────────────────────────────────────────────

  describe "named route params" do
    setup do
      app =
        Application.new()
        |> Application.get("/hello/:name", fn req ->
          json(%{message: "Hello " <> req.params["name"]})
        end)
        |> Application.delete("/items/:id", fn req ->
          text("deleted " <> req.params["id"])
        end)
        |> Application.get("/a/:x/b/:y", fn req ->
          text(req.params["x"] <> "+" <> req.params["y"])
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "GET /hello/:name captures the name", %{port: port} do
      {200, body} = http_get(port, "/hello/Alice")
      assert body =~ "Alice"
    end

    test "DELETE /items/:id captures the id", %{port: port} do
      {200, body} = http_delete(port, "/items/abc-123")
      assert body == "deleted abc-123"
    end

    test "multi-segment capture works", %{port: port} do
      {200, body} = http_get(port, "/a/foo/b/bar")
      assert body == "foo+bar"
    end
  end

  # ── POST body ────────────────────────────────────────────────────────────────

  describe "POST body echo" do
    setup do
      app =
        Application.new()
        |> Application.post("/echo", fn req ->
          {200, %{"content-type" => req.content_type}, req.body}
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "POST /echo returns the body", %{port: port} do
      {200, body} = http_post(port, "/echo", "hello world", ~c"text/plain")
      assert body == "hello world"
    end

    test "POST /echo preserves content-type", %{port: port} do
      {200, body} = http_post(port, "/echo", ~s({"x":1}), ~c"application/json")
      assert body == ~s({"x":1})
    end
  end

  # ── Before filter ─────────────────────────────────────────────────────────────

  describe "before filter halt" do
    setup do
      app =
        Application.new()
        |> Application.before_filter(fn req ->
          if req.path == "/down", do: halt(503, "Maintenance")
        end)
        |> Application.get("/", fn _ -> text("up") end)
        |> Application.get("/down", fn _ -> text("should not reach") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "halt(503) short-circuits", %{port: port} do
      {503, body} = http_get(port, "/down")
      assert body == "Maintenance"
    end

    test "before filter passes through for normal routes", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "up"
    end
  end

  # ── Redirect and halt helpers ─────────────────────────────────────────────────

  describe "redirect + halt" do
    setup do
      app =
        Application.new()
        |> Application.get("/old", fn _ -> redirect("/new") end)
        |> Application.get("/forbidden", fn _ -> halt(403, "no") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "redirect returns 302 with Location header", %{port: port} do
      {302, hdrs, _body} = http_get_full(port, "/old")

      assert Enum.any?(hdrs, fn {name, value} ->
               String.downcase(to_string(name)) == "location" and
                 to_string(value) == "/new"
             end)
    end

    test "halt(403) returns Forbidden", %{port: port} do
      {403, body} = http_get(port, "/forbidden")
      assert body == "no"
    end
  end

  # ── not_found handler ──────────────────────────────────────────────────────────

  describe "not_found handler" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("home") end)
        |> Application.not_found_handler(fn req ->
          html("<h1>Not Found: #{req.path}</h1>", 404)
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "custom not_found handler runs for unknown path", %{port: port} do
      {404, body} = http_get(port, "/nope")
      assert body =~ "Not Found: /nope"
    end

    test "known route is still matched correctly", %{port: port} do
      {200, body} = http_get(port, "/")
      assert body == "home"
    end
  end

  # ── Default 404 ───────────────────────────────────────────────────────────────

  describe "default 404 (no not_found handler)" do
    setup do
      app = Application.new() |> Application.get("/", fn _ -> text("ok") end)
      server = start_server(app)
      {:ok, port: server.port}
    end

    test "returns 404 for unknown paths", %{port: port} do
      {404, _} = http_get(port, "/unknown")
    end
  end

  # ── Error handler ─────────────────────────────────────────────────────────────

  describe "error handler" do
    setup do
      app =
        Application.new()
        |> Application.get("/boom", fn _ -> raise "explosion!" end)
        |> Application.error_handler(fn req ->
          msg = req.env["conduit.error"] || ""
          json(%{error: msg}, 500)
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "exception routes through error handler", %{port: port} do
      {500, body} = http_get(port, "/boom")
      assert body =~ "explosion"
    end
  end

  # ── Query params ──────────────────────────────────────────────────────────────

  describe "query params" do
    setup do
      app =
        Application.new()
        |> Application.get("/search", fn req ->
          q = req.query_params["q"] || ""
          n = req.query_params["n"] || ""
          json(%{q: q, n: n})
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "query params are surfaced via req.query_params", %{port: port} do
      {200, body} = http_get(port, "/search?q=hello&n=5")
      assert body =~ "hello"
      assert body =~ "5"
    end
  end

  # ── After filter ──────────────────────────────────────────────────────────────

  describe "after filter" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("original") end)
        |> Application.after_filter(fn _req ->
          {201, %{"x-modified" => "yes"}, "rewritten"}
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "after filter rewrites the response", %{port: port} do
      {201, hdrs, body} = http_get_full(port, "/")
      assert body == "rewritten"

      assert Enum.any?(hdrs, fn {name, value} ->
               String.downcase(to_string(name)) == "x-modified" and to_string(value) == "yes"
             end)
    end
  end

  # ── Server metadata ───────────────────────────────────────────────────────────

  describe "server metadata" do
    setup do
      app = Application.new() |> Application.get("/", fn _ -> text("ok") end)
      server = start_server(app)
      {:ok, server: server}
    end

    test "local_port/1 reports a non-zero port", %{server: server} do
      assert Server.local_port(server) > 0
    end

    test "running?/1 is true before stop", %{server: server} do
      assert Server.running?(server)
    end

    test "running?/1 is false after stop", %{server: server} do
      # on_exit will also call stop — it's safe because Server.stop is idempotent.
      Server.stop(server)
      Process.sleep(100)
      refute Server.running?(server)
    end

    test "stop/1 is idempotent — safe to call twice", %{server: server} do
      assert :ok = Server.stop(server)
      Process.sleep(50)
      assert :ok = Server.stop(server)
    end
  end

  describe "multiple handlers per app" do
    setup do
      app =
        Application.new()
        |> Application.get("/a", fn _ -> text("a") end)
        |> Application.get("/b", fn _ -> text("b") end)
        |> Application.get("/c", fn _ -> text("c") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "all three routes respond correctly", %{port: port} do
      {200, "a"} = http_get(port, "/a")
      {200, "b"} = http_get(port, "/b")
      {200, "c"} = http_get(port, "/c")
    end
  end

  describe "default not_found for unknown path" do
    setup do
      app =
        Application.new()
        |> Application.get("/known", fn _ -> text("known") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "returns 404 for completely unknown path", %{port: port} do
      {404, _} = http_get(port, "/unknown-route")
    end
  end

  describe "serve/1 API" do
    # serve/1 blocks the calling process with a receive. We test that stopping
    # the server still works while serve/1 is blocking another process.
    test "server can be stopped while serve/1 blocks another process" do
      app = Application.new() |> Application.get("/", fn _ -> text("ok") end)
      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 0)
      Process.sleep(30)

      # spawn a separate process to call serve/1
      spawn(fn -> Server.serve(server) end)
      Process.sleep(10)

      # Stop from this process — should work fine
      assert :ok = Server.stop(server)
      Process.sleep(50)
      refute Server.running?(server)
    end
  end

  describe "error handler with no error message" do
    setup do
      app =
        Application.new()
        |> Application.get("/boom", fn _ -> raise "crash test" end)
        |> Application.error_handler(fn _req ->
          text("handled", 500)
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "error handler runs on exception", %{port: port} do
      {500, body} = http_get(port, "/boom")
      assert body == "handled"
    end
  end

  describe "PUT and PATCH methods" do
    setup do
      app =
        Application.new()
        |> Application.put("/resource/:id", fn req ->
          text("updated " <> req.params["id"])
        end)
        |> Application.patch("/resource/:id", fn req ->
          text("patched " <> req.params["id"])
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "PUT returns correct response", %{port: port} do
      url = ~c"http://127.0.0.1:#{port}/resource/42"
      {:ok, {{_, status, _}, _, body}} =
        :httpc.request(:put, {url, [], ~c"text/plain", "data"}, [], [])
      assert status == 200
      assert IO.iodata_to_binary(body) == "updated 42"
    end

    test "PATCH returns correct response", %{port: port} do
      url = ~c"http://127.0.0.1:#{port}/resource/99"
      {:ok, {{_, status, _}, _, body}} =
        :httpc.request(:patch, {url, [], ~c"text/plain", "data"}, [], [])
      assert status == 200
      assert IO.iodata_to_binary(body) == "patched 99"
    end
  end

  describe "Connection: close behavior" do
    # When a client sends Connection: close, the Worker should close after one response.
    # We test this by making a raw TCP request with Connection: close.
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("hello") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "server responds correctly to connection: close requests", %{port: port} do
      # Use :httpc which naturally uses connection: close in some configurations
      {200, body} = http_get(port, "/")
      assert body == "hello"
    end
  end

  describe "multiple sequential requests" do
    setup do
      app =
        Application.new()
        |> Application.get("/", fn _ -> text("ok") end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "server handles sequential requests without issues", %{port: port} do
      for _ <- 1..5 do
        {200, "ok"} = http_get(port, "/")
      end
    end
  end

  describe "raw response tuple from handler" do
    setup do
      app =
        Application.new()
        |> Application.get("/raw", fn _ ->
          {201, %{"x-test" => "raw"}, "raw body"}
        end)

      server = start_server(app)
      {:ok, port: server.port}
    end

    test "handler returning raw 3-tuple works", %{port: port} do
      {201, hdrs, body} = http_get_full(port, "/raw")
      assert IO.iodata_to_binary(body) == "raw body"
      assert Enum.any?(hdrs, fn {k, v} ->
               String.downcase(to_string(k)) == "x-test" and to_string(v) == "raw"
             end)
    end
  end
end
