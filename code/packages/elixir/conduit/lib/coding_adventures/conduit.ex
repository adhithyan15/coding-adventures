defmodule CodingAdventures.Conduit do
  @moduledoc """
  Conduit — a Sinatra/Express-inspired web framework for Elixir.

  Backed by the same Rust `web-core` engine that powers the Ruby (WEB02),
  Python (WEB03), Lua (WEB04), and TypeScript (WEB05) Conduit ports.
  Elixir handlers run on the BEAM; HTTP I/O runs on a Rust I/O thread;
  `enif_send` ferries requests between them.

  See `WEB06-conduit-elixir.md` in `code/specs/` for the full architecture.

  ## Example

      alias CodingAdventures.Conduit
      alias CodingAdventures.Conduit.{Application, Server}
      import CodingAdventures.Conduit.HandlerContext

      app =
        Application.new()
        |> Application.before_filter(fn req ->
             if req.path == "/down", do: halt(503, "Maintenance")
           end)
        |> Application.get("/", fn _req ->
             html("<h1>Hello from Conduit!</h1>")
           end)
        |> Application.get("/hello/:name", fn req ->
             json(%{message: "Hello " <> req.params["name"]})
           end)

      {:ok, server} = Server.start_link(app, port: 3000)
      Server.serve(server)
  """

  alias CodingAdventures.Conduit.{Application, HandlerContext, Server}

  @doc "Shorthand for `Conduit.Application.new/0`."
  defdelegate application, to: Application, as: :new

  @doc "Shorthand for `Conduit.Server.start_link/2`."
  defdelegate start_link(app, opts), to: Server

  defdelegate html(body), to: HandlerContext
  defdelegate html(body, status), to: HandlerContext
  defdelegate json(value), to: HandlerContext
  defdelegate json(value, status), to: HandlerContext
  defdelegate text(body), to: HandlerContext
  defdelegate text(body, status), to: HandlerContext
  defdelegate respond(status, body), to: HandlerContext
  defdelegate respond(status, body, headers), to: HandlerContext
  defdelegate halt(status), to: HandlerContext
  defdelegate halt(status, body), to: HandlerContext
  defdelegate halt(status, body, headers), to: HandlerContext
  defdelegate redirect(location), to: HandlerContext
  defdelegate redirect(location, status), to: HandlerContext
end
