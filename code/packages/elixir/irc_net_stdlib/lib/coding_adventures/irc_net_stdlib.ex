defmodule CodingAdventures.IrcNetStdlib do
  @moduledoc """
  TCP networking layer for the IRC stack.

  This package provides the concrete TCP transport for the IRC server.
  It uses Elixir's `:gen_tcp` module for sockets and OTP's `GenServer`
  and `Task` primitives for concurrency.

  ## Russian Nesting Doll architecture

      ircd (program)
        |
        +-- irc_net_stdlib (this package)  <-- TCP event loop
        |
        +-- irc_server                     <-- pure IRC state machine
        |
        +-- irc_framing                    <-- CRLF line framing
        |
        +-- irc_proto                      <-- message parsing

  ## Key modules

  * `CodingAdventures.IrcNetStdlib.EventLoop` -- GenServer that manages
    connections and serialises Handler callbacks.

  * `CodingAdventures.IrcNetStdlib.Listener` -- factory for `:gen_tcp`
    server sockets.

  * `CodingAdventures.IrcNetStdlib.Handler` -- behaviour that the `ircd`
    driver layer must implement.

  ## Quick start

      # 1. Start the event loop GenServer.
      {:ok, loop} = CodingAdventures.IrcNetStdlib.start_link()

      # 2. Create a listening socket.
      {:ok, sock} = CodingAdventures.IrcNetStdlib.listen("0.0.0.0", 6667)

      # 3. Start accepting connections (non-blocking).
      {:ok, _pid} = CodingAdventures.IrcNetStdlib.run(loop, sock, MyHandler)

      # 4. Send data to a connected client.
      CodingAdventures.IrcNetStdlib.send_to(loop, conn_id, "PING :irc.local\\r\\n")

      # 5. Graceful shutdown.
      CodingAdventures.IrcNetStdlib.stop(loop)
  """

  alias CodingAdventures.IrcNetStdlib.EventLoop
  alias CodingAdventures.IrcNetStdlib.Listener

  @doc "Start an `EventLoop` GenServer."
  defdelegate start_link(opts \\ []), to: EventLoop

  @doc "Start accepting connections on *listen_socket* and dispatching events to *handler*."
  defdelegate run(loop, listen_socket, handler), to: EventLoop

  @doc "Stop the event loop."
  defdelegate stop(loop), to: EventLoop

  @doc "Send *data* to connection *conn_id*."
  defdelegate send_to(loop, conn_id, data), to: EventLoop

  @doc "Create a TCP listener socket bound to *host*:*port*."
  defdelegate listen(host, port), to: Listener
end
