# irc_net_stdlib (Elixir)

TCP networking layer for the IRC stack.

## Overview

`irc_net_stdlib` is the fourth layer of the IRC stack. It provides the
concrete TCP transport: an accept loop, per-connection worker Tasks, and a
GenServer to manage shared state and serialise Handler callbacks.

## Usage

```elixir
alias CodingAdventures.IrcNetStdlib.{EventLoop, Listener}

# 1. Start the event loop GenServer.
{:ok, loop} = EventLoop.start_link()

# 2. Create a listening socket.
{:ok, sock} = Listener.listen("0.0.0.0", 6667)

# 3. Start accepting connections (non-blocking).
{:ok, _pid} = EventLoop.run(loop, sock, MyHandler)

# 4. Send data to a connected client.
EventLoop.send_to(loop, conn_id, "PING :irc.local\r\n")

# 5. Graceful shutdown.
EventLoop.stop(loop)
```

## Handler Behaviour

Implement `CodingAdventures.IrcNetStdlib.Handler` to receive callbacks:

```elixir
defmodule MyHandler do
  @behaviour CodingAdventures.IrcNetStdlib.Handler

  @impl true
  def on_connect(conn_id, host) do
    IO.puts("Client \#{conn_id} connected from \#{host}")
  end

  @impl true
  def on_data(conn_id, data) do
    IO.puts("Client \#{conn_id} sent: \#{inspect(data)}")
  end

  @impl true
  def on_disconnect(conn_id) do
    IO.puts("Client \#{conn_id} disconnected")
  end
end
```

## Facade

The top-level `CodingAdventures.IrcNetStdlib` module provides a flat API:

```elixir
{:ok, loop} = CodingAdventures.IrcNetStdlib.start_link()
{:ok, sock} = CodingAdventures.IrcNetStdlib.listen("0.0.0.0", 6667)
{:ok, _pid} = CodingAdventures.IrcNetStdlib.run(loop, sock, MyHandler)
CodingAdventures.IrcNetStdlib.send_to(loop, conn_id, data)
CodingAdventures.IrcNetStdlib.stop(loop)
```

## Concurrency Model

- One `EventLoop` GenServer holds the connections map (ETS table) and the
  callback serialisation mutex.
- One accept loop Task blocks on `:gen_tcp.accept/1`.
- One worker Task per connected client reads from the socket and dispatches
  callbacks.
- Handler callbacks are serialised via a mutex-lock mechanism: each worker
  acquires the lock, runs the callback in its own process, then releases.
  This ensures `on_connect`, `on_data`, and `on_disconnect` never run
  concurrently.
- `send_to/3` reads sockets directly from ETS (not through the GenServer) to
  avoid deadlocking when called from within a callback.

## In the Stack

    ircd (program)
      |
      +-- irc_net_stdlib   <-- this package (TCP event loop)
      |
      +-- irc_server       <-- pure IRC state machine
      |
      +-- irc_framing      <-- CRLF framing
      |
      +-- irc_proto        <-- message parsing

## Dependencies

None — this package has no library dependencies beyond Elixir's standard OTP.
