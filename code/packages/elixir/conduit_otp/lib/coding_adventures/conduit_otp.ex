defmodule CodingAdventures.ConduitOtp do
  @moduledoc """
  WEB07 — A pure-OTP Elixir web framework, reimplementation of Conduit.

  ## Quick start

      alias CodingAdventures.ConduitOtp
      alias CodingAdventures.ConduitOtp.{Application, Server}
      import CodingAdventures.ConduitOtp.HandlerContext

      app =
        Application.new()
        |> Application.get("/", fn _req -> html("<h1>Hello from Conduit OTP!</h1>") end)
        |> Application.get("/hello/:name", fn req ->
             json(%{message: "Hello " <> req.params["name"]})
           end)

      {:ok, server} = Server.start_link(app, host: "127.0.0.1", port: 3000)
      Server.serve(server)

  ## OTP architecture

  See the individual module docs for detailed teaching material on each concept:

  - `OtpApplication` — the OTP Application behaviour
  - `OtpSupervisor` — supervisor strategies and restart budgets
  - `Acceptor` — gen_server, passive sockets, send-to-self loops
  - `WorkerSupervisor` — DynamicSupervisor, :temporary restart
  - `Worker` — per-connection gen_server, "let it crash"
  - `HttpParser` — :erlang.decode_packet/3 and BEAM HTTP framing
  - `RouteTable` — Agent for hot-reloadable route storage
  - `Router` — pure path-pattern matching
  - `HandlerContext` — response helpers and throw-based halting
  - `Request` — immutable request struct
  - `Application` — functional DSL for route registration
  - `Server` — the public façade
  """

  @doc "Convenience alias — returns the Application module."
  def application, do: CodingAdventures.ConduitOtp.Application
end
