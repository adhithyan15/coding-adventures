defmodule CodingAdventures.IrcNetStdlib.Listener do
  @moduledoc """
  Factory for creating TCP listener sockets.

  This module wraps `:gen_tcp.listen/2` with sensible options and a clear API.
  The returned socket is used with `EventLoop.run/3`.

  ## Options applied

  - `mode: :binary`     -- Return data as Elixir binaries (not char lists).
  - `packet: :raw`      -- No length framing; raw byte stream. The Framer
                           handles IRC's CRLF framing at a higher layer.
  - `active: false`     -- Passive mode: block in recv() explicitly rather than
                           receiving {:tcp, ...} messages into the mailbox.
  - `reuseaddr: true`   -- Allow immediate port reuse after server exit.

  ## SO_REUSEADDR

  Without `reuseaddr: true`, the OS keeps the port in TIME_WAIT state for
  up to 4 minutes after the server process exits, causing rapid restarts
  to fail with `:eaddrinuse`. Setting this option allows immediate reuse.
  """

  @type listen_socket :: :gen_tcp.socket()

  @default_opts [
    mode: :binary,
    packet: :raw,
    active: false,
    reuseaddr: true
  ]

  @doc """
  Create a TCP listener socket bound to *host*:*port*.

  ## Parameters

  - `host` -- IP address to bind to as a string, e.g. "0.0.0.0" for all
              interfaces or "127.0.0.1" for loopback (useful in tests).
  - `port` -- TCP port number, e.g. 6667. Pass 0 to let the OS assign
              a free ephemeral port (useful in tests).

  ## Returns

  - `{:ok, socket}` on success.
  - `{:error, reason}` on failure (e.g. `:eaddrinuse`, `:eacces`).

  ## Example

      {:ok, sock} = Listener.listen("127.0.0.1", 6667)
      {:ok, loop} = EventLoop.start_link()
      {:ok, _pid} = EventLoop.run(loop, sock, MyHandler)
  """
  @spec listen(String.t(), :inet.port_number()) ::
          {:ok, listen_socket()} | {:error, :inet.posix()}
  def listen(host, port) do
    ip_addr =
      host
      |> String.to_charlist()
      |> :inet.parse_address()
      |> case do
        {:ok, addr} -> addr
        {:error, _} -> raise ArgumentError, "invalid host: #{inspect(host)}"
      end

    opts = [{:ip, ip_addr} | @default_opts]
    :gen_tcp.listen(port, opts)
  end

  @doc """
  Return the local port number of a listener socket.

  Useful when the socket was created with port 0 (OS-assigned port).

  ## Example

      {:ok, sock} = Listener.listen("127.0.0.1", 0)
      port = Listener.port!(sock)  # e.g. 52341
  """
  @spec port!(listen_socket()) :: :inet.port_number()
  def port!(socket) do
    {:ok, port} = :inet.port(socket)
    port
  end

  @doc """
  Close a listener socket.

  After calling `close/1`, any task blocked in `:gen_tcp.accept/1` on this
  socket will receive `{:error, :closed}`.
  """
  @spec close(listen_socket()) :: :ok
  def close(socket) do
    :gen_tcp.close(socket)
    :ok
  end
end
