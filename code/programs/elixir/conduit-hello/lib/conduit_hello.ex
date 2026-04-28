defmodule ConduitHello do
  @moduledoc """
  Conduit demo program — eight routes that exercise every framework feature.

  Run:

      mix deps.get
      mix run --no-halt
      # → server listening on http://127.0.0.1:3000

  ## Routes

  | Method | Path             | Behaviour |
  |--------|------------------|-----------|
  | GET    | `/`              | HTML home |
  | GET    | `/hello/:name`   | JSON greeting using a route param |
  | POST   | `/echo`          | Echoes the request body |
  | GET    | `/redirect`      | 302 to `/` |
  | GET    | `/halt`          | Calls `halt(403, "Forbidden")` |
  | GET    | `/down`          | Triggers the `before` filter to halt(503) |
  | GET    | `/error`         | Raises an exception → routes to `error_handler` |
  | (any)  | `/missing/...`   | Catches via custom `not_found_handler` |

  ## What this teaches

  - Building an Application by chaining functional combinators.
  - Using `before_filter` for cross-cutting concerns (maintenance mode).
  - `halt` and `redirect` as control-flow throws.
  - Setting a custom `not_found_handler` and `error_handler`.
  - Reading route params from `req.params`.
  """

  alias CodingAdventures.Conduit.{Application, Server}
  import CodingAdventures.Conduit.HandlerContext

  @doc "Construct the demo application. Pure function, easy to test."
  def app do
    Application.new()
    |> Application.before_filter(&maintenance/1)
    |> Application.get("/", fn _req ->
      html("""
      <!DOCTYPE html>
      <html>
        <head><title>Conduit Hello</title></head>
        <body>
          <h1>Hello from Conduit (Elixir)!</h1>
          <p>Try <a href="/hello/Adhithya">/hello/Adhithya</a>.</p>
        </body>
      </html>
      """)
    end)
    |> Application.get("/hello/:name", fn req ->
      json(%{message: "Hello " <> req.params["name"]})
    end)
    |> Application.post("/echo", fn req ->
      # Echo the raw body — no JSON parsing required (works without Elixir 1.18+).
      {200, %{"content-type" => req.content_type}, req.body}
    end)
    |> Application.get("/redirect", fn _req -> redirect("/", 301) end)
    |> Application.get("/halt", fn _req -> halt(403, "Forbidden") end)
    |> Application.get("/error", fn _req -> raise "Something went wrong!" end)
    |> Application.not_found_handler(fn req ->
      html("<h1>Not Found: #{req.path}</h1>", 404)
    end)
    |> Application.error_handler(fn req ->
      msg = req.env["conduit.error"] || ""
      json(%{error: "Internal Server Error", detail: msg}, 500)
    end)
    |> Application.put_setting(:app_name, "Conduit Hello (Elixir)")
  end

  @doc """
  Run the server in the foreground. Used by `mix run --no-halt` and
  by the escript entry point.
  """
  def main(args \\ []) do
    {opts, _, _} =
      OptionParser.parse(args,
        switches: [host: :string, port: :integer],
        aliases: [h: :host, p: :port]
      )

    host = opts[:host] || "127.0.0.1"
    port = opts[:port] || 3000

    {:ok, server} = Server.start_link(app(), host: host, port: port)
    IO.puts("Conduit Hello listening on http://#{host}:#{port}")
    Server.serve(server)
  end

  # ── Filters ──────────────────────────────────────────────────────────────

  defp maintenance(req) do
    if req.path == "/down" do
      halt(503, "Under maintenance")
    end
  end
end
